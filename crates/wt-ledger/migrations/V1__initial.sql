-- SPDX-License-Identifier: MIT OR Apache-2.0
-- WalkTunes Ledger schema v1 (docs/DATABASE.md §3). Generated from the DATABASE.md SQL
-- blocks on 2026-11-12. Append-only once released: change the schema with a new V{n} file.

CREATE TABLE machines (
  id            TEXT PRIMARY KEY,          -- MachineId
  name          TEXT NOT NULL,             -- hostname or user label
  is_self       INTEGER NOT NULL DEFAULT 0,
  first_seen_at INTEGER NOT NULL
);

CREATE TABLE roots (
  id         TEXT PRIMARY KEY,
  path       TEXT NOT NULL UNIQUE,         -- absolute, OS-native
  kind       TEXT NOT NULL CHECK (kind IN ('library','inbox','watch')),
  enabled    INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL
);

CREATE TABLE tracks (
  id              TEXT PRIMARY KEY,
  title           TEXT,
  artist          TEXT,
  album           TEXT,
  album_artist    TEXT,
  disc_no         INTEGER,
  track_no        INTEGER,
  year            INTEGER,
  genre           TEXT,
  duration_ms     INTEGER,
  mb_recording_id TEXT,                    -- MusicBrainz recording MBID
  mb_release_id   TEXT,
  acoustid        TEXT,
  fingerprint     BLOB,                    -- compressed chromaprint, optional
  identity_level  INTEGER NOT NULL,        -- see §4 (1 = strongest)
  preferred_file  TEXT REFERENCES files(id),
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_tracks_mbid ON tracks(mb_recording_id);
CREATE INDEX idx_tracks_album ON tracks(album_artist, album, disc_no, track_no);

CREATE TABLE files (
  id            TEXT PRIMARY KEY,
  track_id      TEXT NOT NULL REFERENCES tracks(id),
  root_id       TEXT NOT NULL REFERENCES roots(id),
  rel_path      TEXT NOT NULL,
  size          INTEGER NOT NULL,
  mtime_ms      INTEGER NOT NULL,
  content_hash  TEXT NOT NULL,             -- blake3 of whole file (hex)
  audio_hash    TEXT,                      -- blake3 of audio payload only (§4)
  container     TEXT NOT NULL,             -- flac, mp3, m4a, wav, aiff, dsf, dff, ogg, opus, ape, wma
  codec         TEXT NOT NULL,             -- flac, mp3, aac, alac, pcm, dsd, vorbis, opus, ...
  lossless      INTEGER NOT NULL,
  sample_rate   INTEGER,
  bit_depth     INTEGER,
  channels      INTEGER,
  bitrate_kbps  INTEGER,
  source_kind   TEXT NOT NULL DEFAULT 'unknown'
                CHECK (source_kind IN ('unknown','cd_rip','store','stream_rip','user_marked')),
  missing_since INTEGER,                   -- set when not found on rescan
  scanned_at    INTEGER NOT NULL,
  created_at    INTEGER NOT NULL,
  UNIQUE (root_id, rel_path)
);
CREATE INDEX idx_files_track ON files(track_id);
CREATE INDEX idx_files_hash  ON files(content_hash);
CREATE INDEX idx_files_audio ON files(audio_hash);

CREATE VIRTUAL TABLE tracks_fts USING fts5(
  title, artist, album, album_artist, genre,
  content='tracks', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE artwork (
  hash        TEXT PRIMARY KEY,            -- blake3 of image bytes
  mime        TEXT NOT NULL,
  width       INTEGER NOT NULL,
  height      INTEGER NOT NULL,
  progressive INTEGER NOT NULL,            -- progressive JPEG flag (device compatibility)
  source      TEXT NOT NULL CHECK (source IN ('embedded','folder','caa','user')),
  created_at  INTEGER NOT NULL
);
CREATE TABLE track_artwork (
  track_id     TEXT NOT NULL REFERENCES tracks(id),
  artwork_hash TEXT NOT NULL REFERENCES artwork(hash),
  role         TEXT NOT NULL DEFAULT 'front',
  PRIMARY KEY (track_id, role)
);

CREATE TABLE lyrics (
  track_id   TEXT PRIMARY KEY REFERENCES tracks(id),
  synced     INTEGER NOT NULL,             -- 1 = LRC with timestamps
  body       TEXT NOT NULL,
  source     TEXT NOT NULL CHECK (source IN ('sidecar','embedded','lrclib','user')),
  fetched_at INTEGER NOT NULL
);

CREATE TABLE playlists (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE playlist_items (
  playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
  position    INTEGER NOT NULL,
  track_id    TEXT NOT NULL REFERENCES tracks(id),
  PRIMARY KEY (playlist_id, position)
);

CREATE TABLE devices (
  id               TEXT PRIMARY KEY,       -- DeviceId (also in device.json)
  name             TEXT NOT NULL,          -- "My ZX300"
  profile_id       TEXT NOT NULL,          -- e.g. "sony.classic.zx300"
  model            TEXT,                   -- e.g. "NW-ZX300" (from probe or user)
  usb_vid          INTEGER, usb_pid INTEGER,
  usb_serial       TEXT,                   -- iSerialNumber; groups volumes, re-recognizes after reformat
  firmware_rev     TEXT,                   -- from default-capability.xml (INQUIRY revision is NOT firmware, R-003)
  last_seen_at     INTEGER,
  last_generation  INTEGER,                -- manifest generation we last read
  created_at       INTEGER NOT NULL
);
CREATE TABLE device_volumes (
  device_id   TEXT NOT NULL REFERENCES devices(id),
  role        TEXT NOT NULL CHECK (role IN ('internal','sd','single')),
  lun         INTEGER,                     -- SCSI LUN of this volume on the USB device
  storage_type TEXT,                       -- INTERNAL / CARD from default-capability.xml
  volume_uid  TEXT,                        -- FS UUID/serial when available
  label       TEXT,
  capacity    INTEGER, free INTEGER, fs_type TEXT,
  PRIMARY KEY (device_id, role)
);
-- Mirror of the device manifest, for fast queries. Rebuilt from the manifest on attach.
CREATE TABLE device_files (
  device_id      TEXT NOT NULL REFERENCES devices(id),
  role           TEXT NOT NULL,
  rel_path       TEXT NOT NULL,
  track_id       TEXT REFERENCES tracks(id),
  content_hash   TEXT NOT NULL,
  source_hash    TEXT,                     -- content_hash of the library original
  origin_machine TEXT REFERENCES machines(id),
  size           INTEGER NOT NULL,
  written_at     INTEGER NOT NULL,
  PRIMARY KEY (device_id, role, rel_path)
);
CREATE TABLE sync_selections (
  device_id TEXT NOT NULL REFERENCES devices(id),
  kind      TEXT NOT NULL CHECK (kind IN ('all','artist','album','playlist','track','smart')),
  ref       TEXT NOT NULL,                 -- id or JSON rule for 'smart'
  PRIMARY KEY (device_id, kind, ref)
);
CREATE TABLE sync_runs (
  id          TEXT PRIMARY KEY,
  device_id   TEXT NOT NULL REFERENCES devices(id),
  direction   TEXT NOT NULL CHECK (direction IN ('push','pull','reconcile')),
  status      TEXT NOT NULL CHECK (status IN ('planned','running','done','failed','cancelled','interrupted')),
  plan_json   TEXT NOT NULL,
  stats_json  TEXT,
  started_at  INTEGER, finished_at INTEGER, created_at INTEGER NOT NULL
);

-- Every file mutation performed by Vault. Makes undo and audits possible (SECURITY §3).
CREATE TABLE ops_log (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id      TEXT REFERENCES sync_runs(id),
  op          TEXT NOT NULL CHECK (op IN ('write','replace','move','trash','restore','retag','mkdir')),
  target      TEXT NOT NULL,               -- absolute path at time of op
  aux         TEXT,                        -- source path / trash path
  hash_before TEXT, hash_after TEXT,
  state       TEXT NOT NULL CHECK (state IN ('pending','done','undone','failed')),
  error_code  TEXT,
  at          INTEGER NOT NULL
);

CREATE TABLE proposals (
  id         TEXT PRIMARY KEY,
  track_id   TEXT NOT NULL REFERENCES tracks(id),
  provider   TEXT NOT NULL,                -- musicbrainz, acoustid, caa, lrclib, filename
  kind       TEXT NOT NULL CHECK (kind IN ('tags','artwork','lyrics','identity')),
  payload    TEXT NOT NULL,                -- JSON
  confidence REAL NOT NULL,
  status     TEXT NOT NULL CHECK (status IN ('open','accepted','rejected','applied')),
  created_at INTEGER NOT NULL
);

CREATE TABLE jobs (
  id         TEXT PRIMARY KEY,
  kind       TEXT NOT NULL,
  state      TEXT NOT NULL CHECK (state IN ('queued','running','done','failed','cancelled','interrupted')),
  progress   REAL NOT NULL DEFAULT 0,
  payload    TEXT,
  error_code TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

-- One row per analyzed file version. Keyed by content_hash so a moved file keeps its result,
-- and by analyzer_version so improved detectors trigger a re-check.
CREATE TABLE analyses (
  file_id           TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  content_hash      TEXT NOT NULL,
  analyzer_version  INTEGER NOT NULL,
  mode              TEXT NOT NULL CHECK (mode IN ('preview','full')),
  integrity         TEXT NOT NULL CHECK (integrity IN ('ok','warning','failed','unchecked')),
  md5_status        TEXT NOT NULL CHECK (md5_status IN ('match','mismatch','absent','not_applicable')),
  decode_errors     INTEGER NOT NULL DEFAULT 0,
  samples_expected  INTEGER, samples_decoded INTEGER,     -- truncation check
  verdict           TEXT NOT NULL CHECK (verdict IN
                    ('genuine','suspect','likely_lossy_source','upsampled','padded_bits','not_applicable')),
  confidence        REAL,                                  -- 0..1
  cutoff_hz         INTEGER,                               -- detected lowpass, NULL if none
  effective_bits    INTEGER,                               -- bits actually used
  effective_rate_hz INTEGER,                               -- highest real content x 2
  clipped_samples   INTEGER, fake_stereo INTEGER, peak_dbfs REAL,
  details           TEXT,                                  -- JSON: per-detector evidence
  user_reviewed     INTEGER NOT NULL DEFAULT 0,            -- user marked "checked, OK"
  analyzed_at       INTEGER NOT NULL,
  PRIMARY KEY (file_id, analyzer_version)
);
CREATE INDEX idx_analyses_verdict ON analyses(verdict);

-- Keep the external-content FTS index in step with `tracks` (DATABASE §3.2).
-- `tracks` has no INTEGER PRIMARY KEY, so a plain VACUUM may renumber rowids: after one,
-- run INSERT INTO tracks_fts(tracks_fts) VALUES('rebuild'). Backups use the backup API.
CREATE TRIGGER tracks_fts_ai AFTER INSERT ON tracks BEGIN
  INSERT INTO tracks_fts(rowid, title, artist, album, album_artist, genre)
  VALUES (new.rowid, new.title, new.artist, new.album, new.album_artist, new.genre);
END;
CREATE TRIGGER tracks_fts_ad AFTER DELETE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, album_artist, genre)
  VALUES ('delete', old.rowid, old.title, old.artist, old.album, old.album_artist, old.genre);
END;
CREATE TRIGGER tracks_fts_au AFTER UPDATE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, album_artist, genre)
  VALUES ('delete', old.rowid, old.title, old.artist, old.album, old.album_artist, old.genre);
  INSERT INTO tracks_fts(rowid, title, artist, album, album_artist, genre)
  VALUES (new.rowid, new.title, new.artist, new.album, new.album_artist, new.genre);
END;
