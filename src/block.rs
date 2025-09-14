use crate::hexdisplay::HexDisplayExt;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt;
use std::time::SystemTime;

use crate::record::Record;
use crate::types::{Hashtype, Time};

const BLOCK_INIT_HASH: &str = "BLOCK_INIT_HASH";

const GENESIS_INIT_HASH: &str = "EIN SOF";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    // The index is assigned from the chain
    pub index: u64,
    // previous hash in the chain
    previous_hash: Hashtype,
    // finalization time
    timestamp: Time,
    // the list of data
    pub data: Vec<Record>,
    // hash of the hashes in data, timestamp, index, and previous_hash.
    pub hash: Hashtype,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "idx: {}, timestamp: {}, p_hash: {}, hash: {}",
            self.index,
            self.timestamp,
            self.previous_hash.hex_display(),
            self.hash.hex_display()
        )
    }
}

impl PartialEq for Block {
    // Eq is constant time and relies on the hash to be calculated properly.
    fn eq(&self, other: &Self) -> bool {
        // Normative equality always requires a hash to be filled in.
        self.hash == other.hash && self.hash != "init"
    }
}

impl Eq for Block {}

impl Block {
    // first ever block
    pub(crate) fn genesis() -> Block {
        let starter = Record::genesis_record();
        let previous: Hashtype = GENESIS_INIT_HASH.to_string();
        let mut block = Block {
            index: 0,
            previous_hash: previous,
            timestamp: 0,
            hash: BLOCK_INIT_HASH.to_string(),
            data: vec![starter],
        };
        block.update_hash();
        block
    }
    pub fn new(idx: u64, previous: Hashtype, records: Vec<Record>) -> Block {
        let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => (n.as_secs_f64() * 1000.0) as u64,
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };
        let mut block = Block {
            index: idx,
            previous_hash: previous,
            timestamp: now,
            hash: BLOCK_INIT_HASH.to_string(),
            data: records,
        };
        block.update_hash();
        block
    }

    pub fn previous_hash(&self) -> &Hashtype {
        &self.previous_hash
    }

    pub fn update_hash(&mut self) -> &Hashtype {
        if self.hash == BLOCK_INIT_HASH {
            let result = self.calculate_hash();
            self.hash = result;
        }
        &self.hash
    }

    fn calculate_hash(&self) -> Hashtype {
        let mut hasher = Sha3_256::new();
        hasher.update(self.timestamp.to_be_bytes());
        hasher.update(self.previous_hash.clone());
        for r in &self.data {
            hasher.update(&r.hash);
        }
        hasher.finalize().to_vec().hex_display().to_string()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.data.is_empty() {
            return Err("Block has no records".to_string());
        }
        if self.index == 0 && self.previous_hash != GENESIS_INIT_HASH {
            return Err("Genesis block has invalid previous hash".to_string());
        }
        if self.index > 0 && self.previous_hash.is_empty() {
            return Err("Block has no previous hash".to_string());
        }
        if self.hash == BLOCK_INIT_HASH {
            return Err("Block is not initialized".to_string());
        }

        if self.hash != self.calculate_hash() {
            if self.hash == BLOCK_INIT_HASH {
                return Err("self hash is uncomputed".to_string());
            }
            return Err(format!(
                "Block hash does not match calculated hash (b: '{}' c: '{}')",
                self.hash,
                self.calculate_hash()
            ));
        }

        for d in &self.data {
            d.validate()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_blocks() {
        _ = Block::genesis();
        _ = Record::genesis_record();
        _ = Record::new("sixpence".to_string());
    }
}
