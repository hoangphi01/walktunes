// SPDX-License-Identifier: GPL-3.0-or-later
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
            command: DeviceCommand::Probe { json, all },
        } => probe(&sdk, json, all),
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
        if let Some(m) = &d.model {
            println!("  model      {m}");
        } else if !d.candidates.is_empty() {
            println!("  model?     {}", d.candidates.join(", "));
        }
        for v in &d.volumes {
            let role = match v.role {
                VolumeRole::Internal => "internal",
                VolumeRole::Sd => "sd",
                VolumeRole::Single => "volume",
            };
            println!(
                "  {:<10} {}  ({}, {:.1} GB)",
                role,
                v.mount_point.display(),
                v.fs_type.as_deref().unwrap_or("?"),
                v.capacity as f64 / 1e9
            );
        }
        if !d.protected_root_files.is_empty() {
            println!("  protected  {}", d.protected_root_files.join(", "));
        }
        if !d.evidence.is_empty() {
            println!("  evidence   {}", d.evidence.join("; "));
        }
        for w in &d.warnings {
            println!("  warning    {w}");
        }
    }
}
