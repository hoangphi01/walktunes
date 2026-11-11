// SPDX-License-Identifier: MIT OR Apache-2.0
//! Repository tasks for WalkTunes.
//!
//! - `cargo xtask layers`: enforce the crate layer rules of `docs/ARCHITECTURE.md` §2.
//! - `cargo xtask docs-check`: doc headers, relative links/anchors, error-code registry.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let result = match task.as_str() {
        "layers" => layers(),
        "docs-check" => docs_check(),
        _ => {
            eprintln!("usage: cargo xtask <layers|docs-check>");
            return ExitCode::from(64);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(problems) => {
            for p in &problems {
                eprintln!("✗ {p}");
            }
            eprintln!("{} problem(s)", problems.len());
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

// ---------------------------------------------------------------------------
// layers
// ---------------------------------------------------------------------------

/// Layer of every service crate (ARCHITECTURE §2). Lower layers never depend on higher ones.
const LAYERS: &[(&str, u8)] = &[
    ("wt-core", 0),
    ("wt-config", 1),
    ("wt-watch", 1),
    ("wt-bus", 1),
    ("wt-queue", 1),
    ("wt-i18n", 1),
    ("wt-dsp", 1),
    ("wt-ledger", 2),
    ("wt-vault", 2),
    ("wt-gate", 2),
    ("wt-cache", 2),
    ("wt-crawler", 3),
    ("wt-enrich", 3),
    ("wt-forge", 3),
    ("wt-dock", 3),
    ("wt-inspect", 3),
    ("wt-sync", 4),
    ("wt-sdk", 5),
    ("wt-kit", 5),
    ("wt-console", 6),
    ("wt-cli", 6),
];

/// Crates every crate may use regardless of layer.
const ANYWHERE: &[&str] = &["wt-core", "wt-watch"];
/// `wt-kit` may only depend on these internal crates (plus GPUI later).
const KIT_ALLOWED: &[&str] = &["wt-core", "wt-watch", "wt-i18n"];
/// Front ends talk to the facade and the UI kit only.
const FRONTEND_ALLOWED: &[&str] = &["wt-sdk", "wt-kit", "wt-core", "wt-watch", "wt-i18n"];

fn layers() -> Result<(), Vec<String>> {
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(repo_root())
        .output()
        .map_err(|e| vec![format!("cannot run cargo metadata: {e}")])?;
    let meta: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| vec![format!("bad metadata: {e}")])?;
    let layer: BTreeMap<&str, u8> = LAYERS.iter().copied().collect();
    let mut problems = Vec::new();

    let packages = meta["packages"].as_array().cloned().unwrap_or_default();
    for pkg in &packages {
        let name = pkg["name"].as_str().unwrap_or_default();
        if name == "xtask" {
            continue;
        }
        let Some(&own) = layer.get(name) else {
            problems.push(format!(
                "{name}: crate missing from the layer map (xtask LAYERS + ARCHITECTURE §2)"
            ));
            continue;
        };
        for dep in pkg["dependencies"].as_array().into_iter().flatten() {
            // Dev-dependencies do not create architectural coupling.
            if dep["kind"].as_str() == Some("dev") {
                continue;
            }
            let dname = dep["name"].as_str().unwrap_or_default();
            let Some(&theirs) = layer.get(dname) else {
                continue;
            };
            if name == "wt-kit" && !KIT_ALLOWED.contains(&dname) {
                problems.push(format!(
                    "wt-kit → {dname}: the UI kit must not depend on domain crates"
                ));
            } else if own == 6 && !FRONTEND_ALLOWED.contains(&dname) {
                problems.push(format!(
                    "{name} → {dname}: front ends use wt-sdk / wt-kit only"
                ));
            } else if theirs >= own && !ANYWHERE.contains(&dname) {
                problems.push(format!(
                    "{name} (layer {own}) → {dname} (layer {theirs}): dependencies must point to lower layers"
                ));
            }
        }
    }
    let members: BTreeSet<&str> = packages.iter().filter_map(|p| p["name"].as_str()).collect();
    for (name, _) in LAYERS {
        if !members.contains(name) {
            problems.push(format!("{name}: in the layer map but not in the workspace"));
        }
    }
    if problems.is_empty() {
        println!(
            "✓ layers: {} crates follow ARCHITECTURE §2",
            members.len() - 1
        );
        Ok(())
    } else {
        Err(problems)
    }
}

// ---------------------------------------------------------------------------
// docs-check
// ---------------------------------------------------------------------------

fn docs_check() -> Result<(), Vec<String>> {
    let root = repo_root();
    let mut files = vec![root.join("README.md")];
    files.extend(md_files(&root.join("docs")));
    for krate in dirs(&root.join("crates")) {
        let readme = krate.join("README.md");
        if readme.exists() {
            files.push(readme);
        }
    }

    let mut problems = Vec::new();
    let mut anchors: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    let mut texts = Vec::new();
    for f in &files {
        let Ok(text) = fs::read_to_string(f) else {
            problems.push(format!("cannot read {}", f.display()));
            continue;
        };
        let prose = strip_code_blocks(&text);
        anchors.insert(canon(f), headings(&prose).iter().map(|h| slug(h)).collect());
        texts.push((f.clone(), text, prose));
    }

    for (f, text, prose) in &texts {
        let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        // 1. Header table on every doc in docs/.
        if rel.starts_with("docs/")
            && !text
                .lines()
                .take(12)
                .any(|l| l.starts_with("| Field | Value |") || l.starts_with("| Mục | Giá trị |"))
        {
            problems.push(format!("{rel}: missing header table (DOCS_GUIDE §6)"));
        }
        // 2. Relative links and anchors.
        for target in links(prose) {
            if target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let (path, anchor) = target.split_once('#').unwrap_or((target.as_str(), ""));
            let dest = if path.is_empty() {
                f.clone()
            } else {
                f.parent().unwrap_or(&root).join(path)
            };
            if !dest.exists() {
                problems.push(format!("{rel}: broken link {target}"));
                continue;
            }
            if !anchor.is_empty()
                && let Some(set) = anchors.get(&canon(&dest))
                && !set.contains(anchor)
            {
                problems.push(format!("{rel}: unknown anchor {target}"));
            }
        }
    }

    // 3. Every error code used in code is registered.
    let registry = fs::read_to_string(root.join("docs/ERROR_CONTROL.md")).unwrap_or_default();
    let registered: BTreeSet<String> = registry
        .lines()
        .filter_map(|l| l.strip_prefix("| "))
        .filter_map(|l| l.split(' ').next())
        .filter(|c| c.starts_with("WT-"))
        .map(str::to_owned)
        .collect();
    for (file, code) in codes_in_source(&root.join("crates")) {
        if !registered.contains(&code) {
            problems.push(format!(
                "{}: {code} is not registered in docs/ERROR_CONTROL.md §5",
                file.strip_prefix(&root).unwrap_or(&file).display()
            ));
        }
    }

    if problems.is_empty() {
        println!(
            "✓ docs-check: {} files, links/anchors ok, {} registered error codes",
            texts.len(),
            registered.len()
        );
        Ok(())
    } else {
        Err(problems)
    }
}

fn canon(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}

fn md_files(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    v.sort();
    v
}

fn strip_code_blocks(text: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn headings(prose: &str) -> Vec<String> {
    prose
        .lines()
        .filter_map(|l| {
            let hashes = l.chars().take_while(|c| *c == '#').count();
            (1..=6)
                .contains(&hashes)
                .then(|| l[hashes..].trim().to_owned())
        })
        .filter(|h| !h.is_empty())
        .collect()
}

/// GitHub-style heading anchor.
fn slug(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Targets of inline links `[text](target)`, ignoring inline code spans.
fn links(prose: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in prose.lines() {
        let line = strip_inline_code(line);
        let mut rest = line.as_str();
        while let Some(i) = rest.find("](") {
            let after = &rest[i + 2..];
            if let Some(end) = after.find(')') {
                let target = &after[..end];
                if !target.is_empty() && !target.contains(char::is_whitespace) {
                    out.push(target.to_owned());
                }
                rest = &after[end..];
            } else {
                break;
            }
        }
    }
    out
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(c);
        }
    }
    out
}

/// `ErrorCode::new("SVC", n)` calls and `"WT-SVC-NNNN"` literals in Rust sources.
fn codes_in_source(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&p).unwrap_or_default();
                for code in error_code_calls(&text)
                    .into_iter()
                    .chain(code_literals(&text))
                {
                    found.push((p.clone(), code));
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

fn error_code_calls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("ErrorCode::new(\"") {
        let after = &rest[i + 16..];
        let Some(q) = after.find('"') else { break };
        let svc = &after[..q];
        let tail = after[q + 1..].trim_start_matches([',', ' ']);
        let num: String = tail.chars().take_while(char::is_ascii_digit).collect();
        if !svc.is_empty()
            && svc.chars().all(|c| c.is_ascii_uppercase())
            && let Ok(n) = num.parse::<u16>()
        {
            out.push(format!("WT-{svc}-{n:04}"));
        }
        rest = after;
    }
    out
}

fn code_literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("\"WT-") {
        let after = &rest[i + 1..];
        let token: String = after
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        let parts: Vec<&str> = token.split('-').collect();
        if parts.len() == 3 && parts[2].len() == 4 && parts[2].chars().all(|c| c.is_ascii_digit()) {
            out.push(token.clone());
        }
        rest = &after[token.len().max(1)..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_github() {
        assert_eq!(
            slug("3. Data safety (the #1 risk)"),
            "3-data-safety-the-1-risk"
        );
        assert_eq!(
            slug("Part C — UX laws (decision rules)"),
            "part-c--ux-laws-decision-rules"
        );
        assert_eq!(slug("3.13 `wt-dock` — IoT Core"), "313-wt-dock--iot-core");
    }

    #[test]
    fn finds_error_codes() {
        let src = r#"ErrorCode::new("DOCK", 8) ... code = "WT-DOCK-0007" ... "WT-x""#;
        assert_eq!(error_code_calls(src), vec!["WT-DOCK-0008"]);
        assert_eq!(code_literals(src), vec!["WT-DOCK-0007"]);
    }

    #[test]
    fn links_skip_inline_code() {
        assert_eq!(links("see [a](A.md#x) and `[b](B.md)`"), vec!["A.md#x"]);
    }
}
