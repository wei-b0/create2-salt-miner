use alloy_primitives::hex;
use clap::{Parser, Subcommand};
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::process;

mod display;
mod gpgpu;
mod miner;

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

#[derive(Debug)]
pub struct AppConfig {
    pub factory: [u8; 20],
    pub caller: [u8; 20],
    pub codehash: [u8; 32],
    pub worksize: u32,
    pub pattern: Vec<u8>,
    pub pattern_len: usize,
}

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

            // Parse and validate the pattern
            let pattern_str = unwrapped.pattern.unwrap_or("00".to_string());
            let pattern_bytes = match hex::decode(&pattern_str) {
                Ok(bytes) => bytes,
                Err(_) => {
                    eprintln!("Invalid hex pattern provided: '{}'. Please provide a valid hex string (e.g., '01010101').", pattern_str);
                    process::exit(1);
                }
            };

            if pattern_bytes.is_empty() {
                eprintln!("Pattern cannot be empty. Please provide a valid hex pattern.");
                process::exit(1);
            }

            if pattern_bytes.len() > 20 {
                eprintln!("Pattern is too long ({} bytes). Maximum address length is 20 bytes.", pattern_bytes.len());
                process::exit(1);
            }

            let pattern_len = pattern_bytes.len();

            let app_config = AppConfig {
                factory: hex::decode(
                    unwrapped
                        .factory
                        .unwrap_or("0x0000000000FFe8B47B3e2130213B802212439497".to_string()),
                )
                .unwrap()
                .try_into()
                .unwrap(),
                caller: hex::decode(unwrapped.caller.unwrap_or("0x00".to_string()))
                    .unwrap()
                    .try_into()
                    .unwrap(),
                codehash: hex::decode(unwrapped.codehash.unwrap_or("0x00".to_string()))
                    .unwrap()
                    .try_into()
                    .unwrap(),
                worksize: unwrapped.worksize.unwrap_or(0x4400000 as u32),
                pattern: pattern_bytes,
                pattern_len: pattern_len,
            };

            let display = Display::new();

            start_miner(app_config, display);
        }
        Commands::List {} => {
            gpgpu::list_devices();
        }
    }
}
