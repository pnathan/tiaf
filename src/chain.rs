use crate::block::Block;
use crate::record::Record;
use std::collections::HashMap;
use std::str;

use crate::types::Hashtype;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    // This data structure needs to be a bufferpool.
    data: HashMap<u64, Block>,
    size: u64,

    #[serde(skip)]
    known_block_hashes: Vec<Hashtype>,
    #[serde(skip)]
    known_record_hashes: Vec<Hashtype>,
    // max_verified is the highest index that has been verified.
    //
    #[serde(skip)]
    max_verified: u64,
}

impl PartialEq for Blockchain {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for Blockchain {}

impl Iterator for Blockchain {
    type Item = Block;
    fn next(&mut self) -> Option<Self::Item> {
        ChainIter {
            inner: self,
            idx: 0,
        }
        .next()
    }
}

impl<'a> IntoIterator for &'a Blockchain {
    type Item = Block;
    type IntoIter = ChainIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ChainIter {
            inner: self,
            idx: 0,
        }
    }
}

pub struct ChainIter<'a> {
    inner: &'a Blockchain,
    idx: u64,
}

impl<'a> Iterator for ChainIter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.inner.data.get(&idx).cloned()
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl Blockchain {
    pub fn new() -> Blockchain {
        let mut data = HashMap::new();
        let genesis = Block::genesis();
        data.insert(0, genesis.clone());
        Blockchain {
            data,
            size: 1,
            max_verified: 0,
            known_record_hashes: vec![genesis.data[0].hash.clone()],
            known_block_hashes: vec![genesis.hash],
        }
    }
    pub fn get(&self, idx: u64) -> Option<&Block> {
        self.data.get(&idx)
    }

    pub fn tail(&self, n: u64) -> Vec<&Block> {
        let mut tail: Vec<&Block> = Vec::new();
        if n > self.size {
            return tail;
        }
        let start = std::cmp::max(self.size - n, 0);
        for i in start..self.size {
            tail.push(self.get(i).unwrap());
        }
        tail
    }

    pub fn since(&self, h: &str) -> Result<Vec<&Block>, String> {
        let mut tail: Vec<&Block> = Vec::new();
        let mut idx = self.size - 1;
        while idx > 0 {
            let block = self.get(idx).ok_or("failure to walk chain")?;
            if block.hash == h {
                break;
            }
            tail.push(block);
            idx -= 1;
        }
        tail.reverse();
        Ok(tail)
    }

    pub fn validate(&mut self) -> Result<(), String> {
        if self.size != self.data.len() as u64 {
            return Err("Blockchain size does not match data size".to_string());
        }
        for i in self.max_verified..self.size {
            self.data
                .get(&{ i })
                .ok_or("no data found at index")?
                .validate()?
        }
        self.max_verified = self.size - 1;
        Ok(())
    }
    pub fn full_validate(&self) -> Result<(), String> {
        if self.size != self.data.len() as u64 {
            return Err("Blockchain size does not match data size".to_string());
        }
        for i in self.max_verified..self.size {
            self.data
                .get(&{ i })
                .ok_or("no data found at index")?
                .validate()?
        }
        Ok(())
    }

    pub fn length(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn record_seen(&self, h: &Hashtype) -> bool {
        self.known_record_hashes.contains(h)
    }

    pub fn block_seen(&self, h: &Hashtype) -> bool {
        self.known_block_hashes.contains(h)
    }

    pub fn block_hashes(&self) -> Vec<Hashtype> {
        self.known_block_hashes.clone()
    }

    pub fn append_new_records(&mut self, records: Vec<Record>) -> Result<(), String> {
        let mut records = records.clone();
        records.retain(|r| !self.record_seen(&r.hash));
        self.append_records(records)
    }

    pub fn append_records(&mut self, records: Vec<Record>) -> Result<(), String> {
        let previous_block: &Block = self.get(self.size - 1).ok_or("no data found at index")?;
        let previous_hash = previous_block.hash.clone();

        for record in &records {
            self.known_record_hashes.push(record.hash.clone());
        }

        let block = Block::new(self.size, previous_hash, records);

        self.known_block_hashes.push(block.hash.clone());
        self.data.insert(self.size, block);

        self.size += 1;
        Ok(())
    }

    pub fn append_blocks(&mut self, blocks: Vec<Block>) -> Result<(), String> {
        // check that the first block in the list is the next block in the chain.
        if *blocks[0].previous_hash() != self.get(self.size - 1).unwrap().hash {
            return Err("blockchain does not match".to_string());
        }

        let blocks = blocks.clone();
        for block in &blocks {
            self.known_block_hashes.push(block.hash.clone());
        }
        for block in blocks {
            self.data.insert(self.size, block);
            self.size += 1;
        }
        Ok(())
    }

    pub fn to_json(&self, validation: bool) -> Result<String, String> {
        if validation {
            self.full_validate()?;
        }
        let json = serde_json::to_string_pretty(&self).map_err(|e| e.to_string())?;
        Ok(json)
    }

    pub fn from_json(s: String) -> Result<Blockchain, String> {
        let mut chain: Blockchain = serde_json::from_str(&s).map_err(|e| e.to_string())?;
        chain.validate()?;
        Ok(chain)
    }

    // couple essential outcomes.
    // 1. the candidate is longer than the current chain.
    // 2. the candidate is shorter/same-len than the current chain.
    // 3. candidate chain is invalid.
    pub fn compare_other_chain(&self, candidate: &Blockchain) -> ChainComparison {
        if self.size == 0 {
            return ChainComparison::Invalid("current chain is empty".to_string());
        }
        if candidate.size == 0 {
            return ChainComparison::Invalid("candidate chain is empty".to_string());
        }
        match candidate.full_validate() {
            Ok(_) => {}
            Err(s) => {
                return ChainComparison::Invalid(
                    format!("candidate chain is invalid: {s}").to_string(),
                );
            }
        }

        if candidate.size > self.size {
            ChainComparison::Longer
        } else {
            ChainComparison::ShorterOrSame
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainComparison {
    Longer,
    ShorterOrSame,
    Invalid(String),
}

#[allow(dead_code)]
fn deserialize_blocks<T>(links: Vec<Block>) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    let mut records: Vec<T> = vec![];
    for l in links {
        println!("block: {l:?}");
        for r in l.data {
            if let Ok(record) = serde_json::from_str(r.entry.as_str()) {
                records.push(record)
            }
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use crate::block::Block;
    use crate::chain::{deserialize_blocks, Blockchain};
    use crate::record::Record;
    use rand::distributions::{Alphanumeric, DistString};
    use std::sync::{Arc, Mutex};

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    fn random_string(n: usize) -> String {
        Alphanumeric.sample_string(&mut rand::thread_rng(), n)
    }

    fn generate_records(num_to_get: u64) -> Vec<Record> {
        let suffix = random_string(4);
        let mut records: Vec<Record> = vec![];
        for i in 0..num_to_get {
            let r = Record::new(format!("{} {}", suffix, i));
            r.validate().unwrap();
            records.push(r);
        }
        records
    }

    #[test]
    fn test_deserialize_blocks() {
        #[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
        struct DummyKV {
            entry: String,
            key: u64,
            val: String,
        }
        let mut blocks: Vec<Block> = vec![];
        let mut records: Vec<Record> = vec![];
        let mut data: Vec<DummyKV> = vec![];
        for i in 0..20 {
            // generate random DummyKV, serialize it, and put it into a Record
            let kv = DummyKV {
                entry: random_string(10),
                key: i,
                val: random_string(10),
            };
            // randomly determine if this is going to be a dummy record or something else
            if rand::random::<bool>() {
                let r = Record::new(serde_json::to_string(&kv).unwrap());
                r.validate().unwrap();
                records.push(r);
                data.push(kv);
            } else {
                let r = Record::new(random_string(10));
                r.validate().unwrap();
                records.push(r);
            }
        }
        let block = Block::new(0, "0".to_string(), records);
        blocks.push(block);
        let json = serde_json::to_string_pretty(&blocks).unwrap();
        let blocks: Vec<Block> = serde_json::from_str(&json).unwrap();
        let gotten: Vec<DummyKV> = deserialize_blocks(blocks).unwrap();
        assert_eq!(gotten, data);
    }

    #[test]
    fn test_blockchain_serialization() {
        let bc = Arc::new(Mutex::new(Blockchain::new()));
        let mut b = bc.lock().unwrap();
        b.append_records(generate_records(10)).unwrap();
        b.append_records(generate_records(10)).unwrap();
        let json = b.to_json(true).unwrap();
        let candidate = Blockchain::from_json(json).unwrap();
        assert_eq!(*b, candidate);
    }

    #[test]
    fn test_blockchain_append() {
        let bc = Arc::new(Mutex::new(Blockchain::new()));
        let mut b = bc.lock().unwrap();
        b.append_records(generate_records(10)).unwrap();
        b.append_records(generate_records(10)).unwrap();
        assert_eq!(b.size, 3);
        assert_eq!(b.data.len(), 3);
        assert_eq!(b.data.get(&1).unwrap().data.len(), 10);
        assert_eq!(b.data.get(&2).unwrap().data.len(), 10);
    }
    #[test]
    fn test_blockhashes() {
        let bc = Arc::new(Mutex::new(Blockchain::new()));
        let mut b = bc.lock().unwrap();
        b.append_records(generate_records(10)).unwrap();
        b.append_records(generate_records(10)).unwrap();
        let hashes = b.block_hashes();
        assert_eq!(hashes.len(), 3);
        assert_eq!(hashes[0], b.data.get(&0).unwrap().hash);
        assert_eq!(hashes[1], b.data.get(&1).unwrap().hash);
    }

    #[test]
    fn test_record_hashes() {
        let bc = Arc::new(Mutex::new(Blockchain::new()));
        let mut b = bc.lock().unwrap();
        let r1 = generate_records(10);
        let r2 = generate_records(10);
        b.append_records(r1.clone()).unwrap();
        b.append_records(r2.clone()).unwrap();
        let hashes = b.known_record_hashes.clone();
        assert_eq!(hashes.len(), 21);
        vec![r1, r2].iter().for_each(|r| {
            r.iter().for_each(|r| {
                assert!(hashes.contains(&r.hash));
            })
        });
    }

    #[test]
    fn test_append_new_records() {
        // Verify that we can add records.
        // Verify that we do not double-add records.
        {
            let bc = Arc::new(Mutex::new(Blockchain::new()));
            let mut b = bc.lock().unwrap();
            let r1 = generate_records(10);
            let r2 = generate_records(10);
            b.append_records(r1.clone()).unwrap();
            b.append_records(r2.clone()).unwrap();
            let hashes = b.known_record_hashes.clone();
            assert_eq!(hashes.len(), 21);
            vec![r1, r2].iter().for_each(|r| {
                r.iter().for_each(|r| {
                    assert!(hashes.contains(&r.hash));
                })
            });
        }
        {
            let bc = Arc::new(Mutex::new(Blockchain::new()));
            let mut b = bc.lock().unwrap();
            let r1 = generate_records(10);
            let r2 = generate_records(10);
            b.append_records(r1.clone()).unwrap();
            b.append_new_records(r2.clone()).unwrap();
            let hashes = b.known_record_hashes.clone();
            assert_eq!(hashes.len(), 21);
            vec![r1, r2].iter().for_each(|r| {
                r.iter().for_each(|r| {
                    assert!(hashes.contains(&r.hash));
                })
            });
        }
        {
            let bc = Arc::new(Mutex::new(Blockchain::new()));
            let mut b = bc.lock().unwrap();
            let r1 = generate_records(10);
            let r2 = generate_records(10);
            b.append_records(r1.clone()).unwrap();
            b.append_new_records(r1.clone()).unwrap();
            let hashes = b.known_record_hashes.clone();
            assert_eq!(hashes.len(), 11);
            vec![r1].iter().for_each(|r| {
                r.iter().for_each(|r| {
                    assert!(hashes.contains(&r.hash));
                })
            });
            // verify didn't see what ain'tthere.
            vec![r2].iter().for_each(|r| {
                r.iter().for_each(|r| {
                    assert!(!hashes.contains(&r.hash));
                })
            });
        }
    }
}
