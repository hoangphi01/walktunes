// SPDX-License-Identifier: GPL-3.0-or-later
//! End-to-end tests of `wtunes` against virtual devices (`docs/TESTING.md` §4).

// Test helpers may panic: unwrap is allowed in tests (ERROR_CONTROL §2).
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/devices")
}

fn wtunes(data_dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("wtunes").unwrap();
    cmd.env("WT_DATA_DIR", data_dir.path()).env_remove("WT_LOG");
    cmd
}

fn probe_json(volumes: &[PathBuf]) -> Vec<Value> {
    let data = tempfile::tempdir().unwrap();
    let list: Vec<String> = volumes.iter().map(|p| p.display().to_string()).collect();
    let out = wtunes(&data)
        .env("WT_FAKE_VOLUMES", list.join(","))
        .args(["device", "probe", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice::<Value>(&out.stdout)
        .unwrap()
        .as_array()
        .cloned()
        .unwrap()
}

#[test]
fn paired_zx300_with_sd_card_is_one_device() {
    let zx = fixtures().join("sony.classic.zx300");
    let devices = probe_json(&[zx.join("internal"), zx.join("sd")]);
    assert_eq!(devices.len(), 1);
    let d = &devices[0];
    assert_eq!(d["status"], "paired");
    assert_eq!(d["profile_id"], "sony.classic.zx300");
    assert_eq!(d["model"], "NW-ZX300");
    assert_eq!(d["display_name"], "My ZX300");
    let roles: Vec<&str> = d["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["internal", "sd"]);
    assert!(
        d["protected_root_files"]
            .as_array()
            .unwrap()
            .contains(&Value::from("NW_WM_FW.UPG"))
    );
}

#[test]
fn unpaired_a_series_is_recognized_with_capacity_ordered_candidates() {
    let a40 = fixtures().join("sony.classic.a40/internal");
    let devices = probe_json(&[a40]);
    let d = &devices[0];
    assert_eq!(d["status"], "recognized");
    assert_eq!(d["family"], "sony.classic");
    assert!(d["model"].is_null());
    let top3: Vec<&str> = d["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .take(3)
        .map(|v| v.as_str().unwrap())
        .collect();
    // 29.5 GB usable → the 32 GB models of A30/A40/A50 come first.
    for m in ["NW-A36HN", "NW-A46HN", "NW-A56HN"] {
        assert!(top3.contains(&m), "{m} not in {top3:?}");
    }
}

#[test]
fn info_reports_simd_path_and_paths() {
    let data = tempfile::tempdir().unwrap();
    let out = wtunes(&data).args(["info", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(["avx2", "sse41", "neon", "scalar"].contains(&v["simd"].as_str().unwrap()));
    assert_eq!(v["data_dir"], data.path().display().to_string());
}

#[test]
fn profiles_list_includes_zx_and_a_series() {
    let data = tempfile::tempdir().unwrap();
    let out = wtunes(&data).args(["profiles", "list"]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    for id in [
        "sony.classic.zx300",
        "sony.classic.a30",
        "sony.classic.a40",
        "sony.classic.a50",
    ] {
        assert!(text.contains(id), "{id}");
    }
}

#[test]
fn unknown_command_is_a_usage_error() {
    let data = tempfile::tempdir().unwrap();
    wtunes(&data).arg("frobnicate").assert().code(2);
}
