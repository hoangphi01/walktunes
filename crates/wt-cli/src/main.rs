// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-cli** — the `wtunes` command-line front end (AWS CLI;
//! `docs/ARCHITECTURE.md` §3.17). It talks to `wt-sdk` only.
//!
//! Exit codes follow `docs/ERROR_CONTROL.md` §3: 0 ok, 1 error, 2 fatal, 64 usage.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use wt_core::{Severity, WtError};
use wt_sdk::{MatchStatus, Sdk, SdkError, VolumeRole};

#[derive(Parser)]
#[command(
    name = "wtunes",
    version,
    about = "WalkTunes — offline music library and sync for classic players"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show version, CPU/SIMD path and data locations.
    Info {
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Device profiles.
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    /// Connected players.
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
}

#[derive(Subcommand)]
enum ProfilesCommand {
    /// List built-in device profiles and their models.
    List,
}

#[derive(Subcommand)]
enum DeviceCommand {
    /// Detect connected players (read-only; never writes to the device).
    Probe {
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Also list volumes that do not look like a music player.
        #[arg(long)]
        all: bool,
        /// Dump the raw fingerprints of every volume (for support and community profiles).
        #[arg(long)]
        raw: bool,
        /// With --raw: keep USB serials and volume UUIDs (redacted by default).
        #[arg(long, requires = "raw")]
        show_serial: bool,
    },
}

fn main() -> ExitCode {
    wt_watch::init_tracing();
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {} [{}]", e, e.code());
            if e.severity() == Severity::Fatal {
                ExitCode::from(2)
            } else {
                ExitCode::from(1)
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), SdkError> {
    let sdk = Sdk::open()?;
    match cli.command {
        Command::Info { json } => info(&sdk, json),
        Command::Profiles {
            command: ProfilesCommand::List,
        } => profiles(&sdk),
        Command::Device {
            command:
                DeviceCommand::Probe {
                    json,
                    all,
                    raw,
                    show_serial,
                },
        } => {
            if raw {
                raw_dump(&sdk, show_serial);
            } else {
                probe(&sdk, json, all);
            }
        }
    }
    Ok(())
}

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: cannot serialize output: {e}"),
    }
}

fn info(sdk: &Sdk, json: bool) {
    let i = sdk.system_info();
    if json {
        print_json(&i);
        return;
    }
    println!("WalkTunes {}", i.version);
    println!("platform   {} / {}", i.os, i.arch);
    println!("simd       {}", i.simd);
    println!("machine    {}", i.machine_id);
    println!("data       {}", i.data_dir.display());
    println!("cache      {}", i.cache_dir.display());
    println!("logs       {}", i.log_dir.display());
}

fn profiles(sdk: &Sdk) {
    for p in sdk.profiles().iter().filter(|p| !p.is_abstract) {
        let models: Vec<String> = p
            .models
            .iter()
            .map(|m| {
                let mut s = m.name.clone();
                if !m.aliases.is_empty() {
                    s.push_str(&format!(" ({})", m.aliases.join(", ")));
                }
                s
            })
            .collect();
        println!("{:<20} tier {}  {}", p.id, p.tier, p.name);
        if !models.is_empty() {
            println!("{:<20}         {}", "", models.join(" · "));
        }
    }
}

fn raw_dump(sdk: &Sdk, show_serial: bool) {
    let fps: Vec<_> = sdk
        .devices()
        .fingerprints()
        .into_iter()
        .map(|fp| if show_serial { fp } else { fp.redacted() })
        .collect();
    print_json(&fps);
}

fn gb(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let v = bytes as f64 / 1e9;
    format!("{v:.1} GB")
}

fn khz(hz: u32) -> String {
    if hz >= 1_000_000 {
        format!("{:.1} MHz", f64::from(hz) / 1e6)
    } else {
        format!("{} kHz", f64::from(hz) / 1e3)
    }
}

fn probe(sdk: &Sdk, json: bool, all: bool) {
    let devices: Vec<_> = sdk
        .devices()
        .probe()
        .into_iter()
        .filter(|d| all || d.status != MatchStatus::Generic)
        .collect();
    if json {
        print_json(&devices);
        return;
    }
    if devices.is_empty() {
        println!("No music player detected. Connect the player in USB mass-storage mode.");
        return;
    }
    for d in &devices {
        let status = serde_json::to_value(d.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        println!(
            "● {}  [{} · score {} · {}]",
            d.display_name, status, d.score, d.profile_id
        );
        match (&d.model, &d.model_series) {
            (Some(m), Some(s)) => println!("  model      {m}  (device reports {s})"),
            (Some(m), None) => println!("  model      {m}"),
            (None, _) if !d.candidates.is_empty() => {
                println!("  model?     {}", d.candidates.join(", "))
            }
            _ => {}
        }
        if let Some(fw) = &d.firmware_version {
            println!("  firmware   {fw}");
        }
        if let Some(u) = &d.usb {
            let id = match (u.vid, u.pid) {
                (Some(v), Some(p)) => format!("{v:04x}:{p:04x}"),
                _ => "?".into(),
            };
            let speed = u
                .speed_mbps
                .map(|s| format!(" · {s} Mbit/s"))
                .unwrap_or_default();
            let names = [u.manufacturer.as_deref(), u.product.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            println!("  usb        {id} {names}{speed}");
            if let Some([v, p, r]) = &u.scsi {
                println!("  scsi       {v} / {p} / rev {r}");
            }
        }
        for v in &d.volumes {
            let role = match v.role {
                VolumeRole::Internal => "internal",
                VolumeRole::Sd => "sd",
                VolumeRole::Single => "volume",
            };
            let fs = v.fs_name.as_deref().or(v.fs_type.as_deref()).unwrap_or("?");
            let free = v
                .free
                .map(|f| format!(", {} free", gb(f)))
                .unwrap_or_default();
            let label = v
                .label
                .as_deref()
                .map(|l| format!(" \"{l}\""))
                .unwrap_or_default();
            println!(
                "  {role:<10} {}{label}  ({fs}, {}{free})",
                v.mount_point.display(),
                gb(v.capacity)
            );
        }
        if let Some(cap) = &d.capability {
            let formats: Vec<String> = {
                let mut seen = Vec::new();
                for f in &cap.audio {
                    let lim = match (f.max_sample_rate, f.max_bits) {
                        (Some(r), Some(b)) if b > 1 => format!(" ≤{}/{}-bit", khz(r), b),
                        (Some(r), _) => format!(" ≤{}", khz(r)),
                        _ => String::new(),
                    };
                    let item = format!("{}{lim}", f.extension);
                    if !seen.iter().any(|s: &String| {
                        s.starts_with(&format!("{} ", f.extension)) || *s == f.extension
                    }) {
                        seen.push(item);
                    }
                }
                seen
            };
            println!("  formats    {}", formats.join(", "));
            let pl: Vec<String> = cap
                .playlists
                .iter()
                .map(|p| format!("{} (.{})", p.id, p.extension))
                .collect();
            let ly: Vec<String> = cap
                .lyrics
                .iter()
                .map(|l| format!(".{}", l.extension))
                .collect();
            println!(
                "  playlists  {}   lyrics {}   music folder {}",
                if pl.is_empty() {
                    "-".into()
                } else {
                    pl.join(", ")
                },
                if ly.is_empty() {
                    "-".into()
                } else {
                    ly.join(", ")
                },
                cap.music_path.as_deref().unwrap_or("?")
            );
            if let Some((w, h)) = cap.display {
                println!("  display    {w}×{h}");
            }
        }
        if !d.protected_root_files.is_empty() {
            println!("  protected  {}", d.protected_root_files.join(", "));
        }
        for n in &d.notes {
            println!("  note       {n}");
        }
        if !d.evidence.is_empty() {
            println!("  evidence   {}", d.evidence.join("; "));
        }
        for w in &d.warnings {
            println!("  warning    {w}");
        }
    }
}
