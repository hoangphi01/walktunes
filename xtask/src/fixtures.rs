// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask fixtures`: generates synthetic test audio with ffmpeg
//! (`docs/TESTING.md` §3). The recipe is `xtask/src/fixtures/media.toml`.
//!
//! Output goes to `fixtures/media/` (git-ignored). Generation is skipped when the recipe
//! has not changed since the last run, unless `--force` is given. After generating,
//! `ffprobe` checks that every title tag and every embedded picture made it into the file.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

const RECIPE: &str = include_str!("fixtures/media.toml");
/// Bump when the generator itself changes in a way that alters the output.
const GENERATOR_VERSION: u32 = 1;

#[derive(Deserialize)]
struct Recipe {
    art: Vec<Art>,
    clip: Vec<Clip>,
}

#[derive(Deserialize)]
struct Art {
    id: String,
    kind: String,
    size: u32,
    color: String,
}

#[derive(Deserialize)]
struct Clip {
    path: String,
    codec: String,
    rate: u32,
    bits: Option<u32>,
    bitrate: Option<String>,
    vbr_quality: Option<u32>,
    seconds: u32,
    signal: String,
    art: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

pub fn run(args: &[String]) -> Result<(), Vec<String>> {
    let force = args.iter().any(|a| a == "--force");
    let recipe: Recipe = toml::from_str(RECIPE).map_err(|e| vec![format!("recipe: {e}")])?;
    let out = crate::repo_root().join("fixtures/media");
    let stamp = out.join(".recipe-hash");
    let want = blake3::hash(format!("{GENERATOR_VERSION}\n{RECIPE}").as_bytes())
        .to_hex()
        .to_string();
    if !force && fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == want) {
        println!(
            "✓ fixtures: {} clips up to date in {}",
            recipe.clip.len(),
            out.display()
        );
        return Ok(());
    }
    for tool in ["ffmpeg", "ffprobe"] {
        if Command::new(tool).arg("-version").output().is_err() {
            return Err(vec![format!(
                "{tool} not found: install ffmpeg 6+ (macOS: brew install ffmpeg)"
            )]);
        }
    }

    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| vec![format!("{}: {e}", out.display())])?;
    }
    let art_dir = out.join("_art");
    fs::create_dir_all(&art_dir).map_err(|e| vec![format!("{}: {e}", art_dir.display())])?;

    let mut art_files = BTreeMap::new();
    for a in &recipe.art {
        let path = make_art(a, &art_dir).map_err(|e| vec![format!("art {}: {e}", a.id)])?;
        art_files.insert(a.id.clone(), path);
    }

    let mut problems = Vec::new();
    for c in &recipe.clip {
        let target = out.join(&c.path);
        let art = match &c.art {
            Some(id) => match art_files.get(id) {
                Some(p) => Some(p.as_path()),
                None => {
                    problems.push(format!("{}: unknown art id `{id}`", c.path));
                    continue;
                }
            },
            None => None,
        };
        if let Err(e) = make_clip(c, art, &target) {
            problems.push(format!("{}: {e}", c.path));
            continue;
        }
        if let Err(e) = verify(c, &target) {
            problems.push(format!("{}: {e}", c.path));
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    fs::write(&stamp, &want).map_err(|e| vec![format!("{}: {e}", stamp.display())])?;
    println!(
        "✓ fixtures: generated {} clips and {} artwork files in {}",
        recipe.clip.len(),
        recipe.art.len(),
        out.display()
    );
    Ok(())
}

fn rgb(hex: &str) -> Result<[u8; 3], String> {
    let v = u32::from_str_radix(hex, 16).map_err(|e| format!("color {hex}: {e}"))?;
    Ok([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn make_art(a: &Art, dir: &Path) -> Result<PathBuf, String> {
    match a.kind.as_str() {
        "jpeg" | "png" => {
            let ext = if a.kind == "png" { "png" } else { "jpg" };
            let path = dir.join(format!("{}.{ext}", a.id));
            let src = format!("color=c=0x{}:s={}x{}", a.color, a.size, a.size);
            ffmpeg(&["-f", "lavfi", "-i", &src, "-frames:v", "1"], &path)?;
            Ok(path)
        }
        "jpeg-progressive" => {
            // ffmpeg's mjpeg encoder cannot write progressive JPEG; jpeg-encoder can.
            let path = dir.join(format!("{}.jpg", a.id));
            let [r, g, b] = rgb(&a.color)?;
            let n = a.size as usize;
            let mut px = Vec::with_capacity(n * n * 3);
            for y in 0..n {
                for x in 0..n {
                    let shade = ((x + y) * 255 / (2 * n)) as u8;
                    px.extend_from_slice(&[
                        r.saturating_add(shade / 4),
                        g,
                        b.saturating_add(shade / 2),
                    ]);
                }
            }
            let size = u16::try_from(a.size).map_err(|_| "image too large".to_owned())?;
            let mut enc = jpeg_encoder::Encoder::new_file(&path, 85).map_err(|e| e.to_string())?;
            enc.set_progressive(true);
            enc.encode(&px, size, size, jpeg_encoder::ColorType::Rgb)
                .map_err(|e| e.to_string())?;
            Ok(path)
        }
        other => Err(format!("unknown art kind `{other}`")),
    }
}

fn make_clip(c: &Clip, art: Option<&Path>, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let source = match c.signal.split_once(':') {
        Some(("sine", hz)) => format!(
            "sine=frequency={hz}:sample_rate={}:duration={}",
            c.rate, c.seconds
        ),
        None if c.signal == "noise" => {
            format!(
                "anoisesrc=color=pink:amplitude=0.3:sample_rate={}:duration={}",
                c.rate, c.seconds
            )
        }
        _ => return Err(format!("unknown signal `{}`", c.signal)),
    };
    let mut args: Vec<String> = vec!["-f".into(), "lavfi".into(), "-i".into(), source];
    if let Some(a) = art {
        args.extend(["-i".into(), a.display().to_string()]);
        args.extend(["-map".into(), "0:a".into(), "-map".into(), "1:v".into()]);
        args.extend([
            "-c:v".into(),
            "copy".into(),
            "-disposition:v".into(),
            "attached_pic".into(),
        ]);
    }
    args.extend(["-ac".into(), "2".into(), "-ar".into(), c.rate.to_string()]);
    let bits = c.bits.unwrap_or(16);
    let codec: &[&str] = match (c.codec.as_str(), bits) {
        ("flac", 24) => &[
            "-c:a",
            "flac",
            "-sample_fmt",
            "s32",
            "-bits_per_raw_sample",
            "24",
        ],
        ("flac", _) => &["-c:a", "flac", "-sample_fmt", "s16"],
        ("alac", 24) => &["-c:a", "alac", "-sample_fmt", "s32p"],
        ("alac", _) => &["-c:a", "alac", "-sample_fmt", "s16p"],
        ("wav", 24) => &["-c:a", "pcm_s24le"],
        ("wav", _) => &["-c:a", "pcm_s16le"],
        ("aiff", 24) => &["-c:a", "pcm_s24be", "-write_id3v2", "1"],
        ("aiff", _) => &["-c:a", "pcm_s16be", "-write_id3v2", "1"],
        ("aac", _) => &["-c:a", "aac"],
        ("mp3", _) => &["-c:a", "libmp3lame", "-id3v2_version", "3"],
        (other, _) => return Err(format!("unknown codec `{other}`")),
    };
    args.extend(codec.iter().map(|s| (*s).to_owned()));
    match (&c.bitrate, c.vbr_quality) {
        (Some(b), _) => args.extend(["-b:a".into(), b.clone()]),
        (None, Some(q)) => args.extend(["-q:a".into(), q.to_string()]),
        (None, None) => {}
    }
    for (k, v) in &c.tags {
        args.extend(["-metadata".into(), format!("{k}={v}")]);
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    ffmpeg(&refs, target)
}

fn ffmpeg(args: &[&str], out: &Path) -> Result<(), String> {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .arg(out)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// Checks the title tag and the embedded picture with ffprobe.
fn verify(c: &Clip, path: &Path) -> Result<(), String> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    if let Some(want) = c.tags.get("title") {
        // Tag keys differ in case between containers (TITLE in FLAC, title in MP4).
        let tags = v["format"]["tags"].as_object();
        let got = tags.and_then(|t| {
            t.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("title"))
                .and_then(|(_, v)| v.as_str())
        });
        if got != Some(want.as_str()) {
            return Err(format!("title tag is {got:?}, expected {want:?}"));
        }
    }
    if c.art.is_some() {
        let has_pic = v["streams"].as_array().is_some_and(|s| {
            s.iter()
                .any(|st| st["disposition"]["attached_pic"].as_i64() == Some(1))
        });
        if !has_pic {
            return Err("embedded picture missing".into());
        }
    }
    Ok(())
}
