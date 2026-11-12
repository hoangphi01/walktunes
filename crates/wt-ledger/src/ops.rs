// SPDX-License-Identifier: MIT OR Apache-2.0
//! The operation journal (`ops_log`, `docs/DATABASE.md` §3.5; SECURITY S-5).

use std::path::PathBuf;

use rusqlite::{Row, params};
use wt_core::{
    ContentHash, ErrorCode, JournalError, OpEntry, OpId, OpKind, OpRecord, OpState, OpsJournal,
};

use crate::{Ledger, LedgerError, now_ms};

/// Read access to `ops_log`, returned by [`Ledger::ops`].
#[derive(Debug, Clone, Copy)]
pub struct Ops<'a> {
    pub(crate) ledger: &'a Ledger,
}

const COLUMNS: &str = "id, run_id, op, target, aux, hash_before, hash_after, state, error_code, at";

fn invalid(column: &'static str, value: &str) -> LedgerError {
    LedgerError::InvalidValue {
        column,
        value: value.to_owned(),
    }
}

fn parse_hash(column: &'static str, v: Option<String>) -> Result<Option<ContentHash>, LedgerError> {
    v.map(|s| s.parse().map_err(|_| invalid(column, &s)))
        .transpose()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<OpEntry, LedgerError>> {
    let id: i64 = row.get(0)?;
    let run_id: Option<String> = row.get(1)?;
    let op: String = row.get(2)?;
    let target: String = row.get(3)?;
    let aux: Option<String> = row.get(4)?;
    let hash_before: Option<String> = row.get(5)?;
    let hash_after: Option<String> = row.get(6)?;
    let state: String = row.get(7)?;
    let error_code: Option<String> = row.get(8)?;
    let at: i64 = row.get(9)?;
    Ok((|| {
        Ok(OpEntry {
            id: OpId(id),
            run_id: run_id
                .map(|s| s.parse().map_err(|_| invalid("ops_log.run_id", &s)))
                .transpose()?,
            op: OpKind::parse(&op).ok_or_else(|| invalid("ops_log.op", &op))?,
            target: PathBuf::from(target),
            aux: aux.map(PathBuf::from),
            hash_before: parse_hash("ops_log.hash_before", hash_before)?,
            hash_after: parse_hash("ops_log.hash_after", hash_after)?,
            state: OpState::parse(&state).ok_or_else(|| invalid("ops_log.state", &state))?,
            error_code,
            at,
        })
    })())
}

impl Ledger {
    fn query_ops(&self, sql: &str, limit: i64) -> Result<Vec<OpEntry>, LedgerError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([limit], from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row??);
        }
        Ok(out)
    }
}

impl Ops<'_> {
    /// The most recent `limit` entries, newest first.
    ///
    /// # Errors
    /// On an SQLite failure or an invalid stored value.
    pub fn recent(&self, limit: u32) -> Result<Vec<OpEntry>, LedgerError> {
        self.ledger.query_ops(
            &format!("SELECT {COLUMNS} FROM ops_log ORDER BY id DESC LIMIT ?1"),
            i64::from(limit),
        )
    }
}

impl OpsJournal for Ledger {
    fn begin(&self, rec: &OpRecord) -> Result<OpId, JournalError> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO ops_log (run_id, op, target, aux, hash_before, state, at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
            params![
                rec.run_id.map(|r| r.to_string()),
                rec.op.as_str(),
                rec.target.to_string_lossy(),
                rec.aux.as_ref().map(|a| a.to_string_lossy().into_owned()),
                rec.hash_before.map(|h| h.to_hex()),
                now_ms(),
            ],
        )
        .map_err(LedgerError::from)?;
        Ok(OpId(conn.last_insert_rowid()))
    }

    fn finish(
        &self,
        id: OpId,
        state: OpState,
        hash_after: Option<&ContentHash>,
        error_code: Option<ErrorCode>,
    ) -> Result<(), JournalError> {
        self.conn()
            .execute(
                "UPDATE ops_log SET state = ?2, hash_after = ?3, error_code = ?4, at = ?5
                 WHERE id = ?1",
                params![
                    id.0,
                    state.as_str(),
                    hash_after.map(ContentHash::to_hex),
                    error_code.map(|c| c.to_string()),
                    now_ms(),
                ],
            )
            .map_err(LedgerError::from)?;
        Ok(())
    }

    fn pending(&self) -> Result<Vec<OpEntry>, JournalError> {
        Ok(self.query_ops(
            &format!("SELECT {COLUMNS} FROM ops_log WHERE state = 'pending' ORDER BY id LIMIT ?1"),
            i64::MAX,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn begin_finish_pending() {
        let ledger = Ledger::open_in_memory().unwrap();
        let rec = OpRecord {
            run_id: None,
            op: OpKind::Write,
            target: PathBuf::from("/Volumes/WALKMAN/MUSIC/a.flac"),
            aux: Some(PathBuf::from("/Volumes/WALKMAN/MUSIC/.a.flac.wt-tmp-x")),
            hash_before: None,
        };
        let a = ledger.begin(&rec).unwrap();
        let b = ledger.begin(&rec).unwrap();
        let h = ContentHash::from_bytes([7; 32]);
        ledger.finish(a, OpState::Done, Some(&h), None).unwrap();
        ledger
            .finish(b, OpState::Failed, None, Some(ErrorCode::new("VAULT", 3)))
            .unwrap();
        assert!(ledger.pending().unwrap().is_empty());
        let recent = ledger.ops().recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, b);
        assert_eq!(recent[0].error_code.as_deref(), Some("WT-VAULT-0003"));
        assert_eq!(recent[1].hash_after, Some(h));
        assert_eq!(recent[1].aux, rec.aux);

        let c = ledger.begin(&rec).unwrap();
        let pending = ledger.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, c);
        assert_eq!(pending[0].state, OpState::Pending);
    }
}
