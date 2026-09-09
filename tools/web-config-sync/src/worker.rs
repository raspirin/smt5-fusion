use std::{fs, path::Path};

use crate::Result;

const LOADER: &str = "smt5-fusion-worker_loader.js";

pub const FILES: [&str; 3] = [
    "smt5-fusion-worker.js",
    "smt5-fusion-worker_bg.wasm",
    LOADER,
];

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

pub struct WorkerAssets {
    directory: String,
}

impl WorkerAssets {
    pub fn read(staging: &Path) -> Result<Self> {
        let mut hash = FNV_OFFSET_BASIS;
        for name in FILES {
            let bytes = fs::read(staging.join(name))
                .map_err(|error| format!("Failed to read Worker asset {name}: {error}"))?;
            if bytes.is_empty() {
                return Err(format!("Empty Worker asset: {name}").into());
            }
            for part in [name.as_bytes(), bytes.as_slice()] {
                hash = fnv1a(hash, &(part.len() as u64).to_le_bytes());
                hash = fnv1a(hash, part);
            }
        }
        Ok(Self {
            directory: format!("worker/{hash:016x}"),
        })
    }

    pub fn entry_url(&self) -> String {
        format!("./{}/{LOADER}", self.directory)
    }

    pub fn relocate(self, staging: &Path) -> Result<()> {
        fs::create_dir_all(staging.join("worker"))?;
        let destination = staging.join(self.directory);
        fs::create_dir(&destination)?;
        for name in FILES {
            fs::rename(staging.join(name), destination.join(name))?;
        }
        Ok(())
    }
}

fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_known_vectors() {
        for (text, expected) in [
            ("", 0xcbf29ce484222325),
            ("a", 0xaf63dc4c8601ec8c),
            ("hello", 0xa430d84680aabd0b),
            ("foobar", 0x85944171f73967e8),
        ] {
            assert_eq!(fnv1a(FNV_OFFSET_BASIS, text.as_bytes()), expected);
        }
    }
}
