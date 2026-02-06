use alloy_primitives::{hex, Address, Keccak256};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub factory: [u8; 20],
    pub caller: [u8; 20],
    pub codehash: [u8; 32],
    pub worksize: u32,
    pub pattern: Vec<u8>,
    pub pattern_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundResult {
    pub salt: String,
    pub address: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawConfig {
    pub factory: String,
    pub caller: String,
    pub codehash: String,
    pub worksize: u32,
    pub pattern: String,
}

pub fn parse_config(raw: RawConfig) -> Result<MinerConfig, String> {
    let factory = parse_fixed_hex::<20>(&raw.factory, "factory")?;
    let caller = parse_fixed_hex::<20>(&raw.caller, "caller")?;
    let codehash = parse_fixed_hex::<32>(&raw.codehash, "codehash")?;
    let pattern = parse_pattern(&raw.pattern)?;
    let pattern_len = pattern.len();

    Ok(MinerConfig {
        factory,
        caller,
        codehash,
        worksize: raw.worksize,
        pattern,
        pattern_len,
    })
}

fn parse_pattern(input: &str) -> Result<Vec<u8>, String> {
    let pattern_str = strip_0x(input);
    if pattern_str.is_empty() {
        return Err("Pattern cannot be empty.".to_string());
    }
    let bytes = hex::decode(pattern_str)
        .map_err(|_| format!("Invalid hex pattern provided: '{}'.", input))?;
    if bytes.is_empty() {
        return Err("Pattern cannot be empty.".to_string());
    }
    if bytes.len() > 20 {
        return Err(format!(
            "Pattern is too long ({} bytes). Maximum address length is 20 bytes.",
            bytes.len()
        ));
    }
    Ok(bytes)
}

fn parse_fixed_hex<const N: usize>(input: &str, name: &str) -> Result<[u8; N], String> {
    let data = hex::decode(strip_0x(input))
        .map_err(|_| format!("Invalid hex string for {}: '{}'.", name, input))?;
    if data.len() != N {
        return Err(format!(
            "Invalid length for {}. Expected {} bytes, got {} bytes.",
            name,
            N,
            data.len()
        ));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&data);
    Ok(out)
}

fn strip_0x(input: &str) -> &str {
    input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")).unwrap_or(input)
}

pub fn run_batch(
    config: &MinerConfig,
    seed: u64,
    worker_id: u32,
    batch_size: u32,
) -> (Vec<FoundResult>, u32) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ (worker_id as u64));
    let salt: [u8; 4] = rng.gen();
    let start_nonce: u64 = rng.gen();

    let mut found = Vec::new();

    for i in 0..batch_size {
        let nonce = start_nonce.wrapping_add(i as u64).to_le_bytes();
        let mut message = [0u8; 85];
        message[0] = 0xff;
        message[1..21].copy_from_slice(&config.factory);
        message[21..41].copy_from_slice(&config.caller);
        message[41..45].copy_from_slice(&salt);
        message[45..53].copy_from_slice(&nonce);
        message[53..].copy_from_slice(&config.codehash);

        let mut hash = Keccak256::new();
        hash.update(&message);
        let mut res = [0u8; 32];
        hash.finalize_into(&mut res);

        let address = <&Address>::try_from(&res[12..]).unwrap();

        let mut matches = true;
        for idx in 0..config.pattern_len {
            if address[idx] != config.pattern[idx] {
                matches = false;
                break;
            }
        }

        if matches {
            let salt_hex = format!(
                "0x{}{}{}",
                hex::encode(config.caller),
                hex::encode(salt),
                hex::encode(nonce)
            );

            found.push(FoundResult {
                salt: salt_hex,
                address: address.to_string(),
                pattern: format!("0x{}", hex::encode(&config.pattern)),
            });
        }
    }

    (found, batch_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pattern_rejects_empty() {
        assert!(parse_pattern("").is_err());
        assert!(parse_pattern("0x").is_err());
    }

    #[test]
    fn parse_pattern_rejects_too_long() {
        let long = "00".repeat(21);
        assert!(parse_pattern(&long).is_err());
    }

    #[test]
    fn parse_fixed_hex_enforces_length() {
        let res = parse_fixed_hex::<20>("0xdeadbeef", "factory");
        assert!(res.is_err());
    }
}
