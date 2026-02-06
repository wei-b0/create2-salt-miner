use clap::{Parser, Subcommand};
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::process;

mod core;
mod display;
mod gpgpu;
mod miner;

use crate::core::{parse_config, RawConfig};
pub use display::Display;
pub use miner::start_miner;

#[derive(Parser, Debug, Serialize, Deserialize)]
struct MineArgs {
    /// Factory Address
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    factory: Option<String>,

    /// Caller Address
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    caller: Option<String>,

    /// Initcode Hash
    #[arg(short = 'i', long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    codehash: Option<String>,

    /// Work Size
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    worksize: Option<u32>,

    /// Hex pattern to match at start of address (e.g., '01010101')
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pattern: Option<String>,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
enum Commands {
    /// Start Create2 Salt Miner
    Mine(MineArgs),
    /// List available OpenCL Platforms (& Devices), including default
    List {},
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(name = "Salty", author, version, about, long_about = None)]
struct CLI {
    #[command(subcommand)]
    mode: Commands,
}

#[cfg(feature = "cli")]
fn main() {
    let cli = CLI::parse();

    match &cli.mode {
        Commands::Mine(args) => {
            let unwrapped: MineArgs = Figment::new()
                .merge(Toml::file("salty.toml"))
                .merge(Serialized::defaults(args))
                .extract()
                .unwrap();

            println!("{:#?}", unwrapped);

            if unwrapped.caller.is_none() || unwrapped.codehash.is_none() {
                eprintln!("Insufficient arguments provided. Please see --help for usage.");
                process::exit(1);
            }

            let raw = RawConfig {
                factory: unwrapped
                    .factory
                    .unwrap_or("0x0000000000FFe8B47B3e2130213B802212439497".to_string()),
                caller: unwrapped.caller.unwrap_or("0x00".to_string()),
                codehash: unwrapped.codehash.unwrap_or("0x00".to_string()),
                worksize: unwrapped.worksize.unwrap_or(0x4400000 as u32),
                pattern: unwrapped.pattern.unwrap_or("00".to_string()),
            };

            let app_config = match parse_config(raw) {
                Ok(cfg) => cfg,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            let display = Display::new();

            start_miner(app_config, display);
        }
        Commands::List {} => {
            gpgpu::list_devices();
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature is disabled. Enable it with `--features cli`.");
}
