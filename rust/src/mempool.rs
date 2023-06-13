use std::collections::HashSet;
use crate::record::{Record};

/// MemPool is a locally-unique group of Records that gets shared across different nodes.
pub struct MemPool {
    data: HashSet<Record>,
    bound: u32,
}

#[derive(Debug, PartialEq)]
pub enum MemPoolError {
    Full,
}

impl MemPool {
    pub fn new(max_size: usize) -> MemPool {
        MemPool {
            data: HashSet::new(),
            bound: max_size as u32,
        }
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }

    pub fn contains(&self, r: &Record) -> bool {
        self.data.contains(r)
    }

    pub fn contents(&self) -> &HashSet<Record> {
        &self.data
    }

    // puts unique r into self. dupes are ignored.
    pub fn put(&mut self, r: Record) -> Result<(), MemPoolError> {
        if self.data.len() as u32 >= self.bound {
            return Err(MemPoolError::Full);
        }
        self.data.insert(r);
        Ok(())
    }

    pub fn reset(&mut self) -> HashSet<Record> {
        self.data.drain().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Record;
    use super::*;

    #[test]
    fn test_mempool() {
        let mut mempool = MemPool::new(10);
        let r = Record::new("hello".to_string());
        mempool.put(r.clone()).unwrap();
        assert_eq!(mempool.length(), 1);
        assert!(mempool.contains(&r));
        mempool.reset();
        assert_eq!(mempool.length(), 0);
    }

    #[test]
    fn test_mempool_full() {
        let mut mempool = MemPool::new(10);
        for i in 0..10 {
            let r = Record::new(i.to_string());
            mempool.put(r).unwrap();
        }
        assert_eq!(mempool.length(), 10);
        let r = Record::new("test".to_string());
        assert_eq!(mempool.put(r), Err(MemPoolError::Full));
    }

    #[test]
    fn test_mempool_empty() {
        let mut mempool = MemPool::new(10);
        assert_eq!(mempool.length(), 0);
        assert_eq!(mempool.reset(), HashSet::<Record>::new());
    }

    #[test]
    fn test_mempool_max_top() {
        let mut mempool = MemPool::new(10);
        let mut recs = Vec::<Record>::new();
        for i in 0..10 {
            let r = Record::new(i.to_string());
            recs.push(r.clone());
            mempool.put(r).unwrap();
        }

        assert_eq!(mempool.length(), 10);
        let gotten: HashSet<Record> = recs.drain(0..).collect();
        assert_eq!(mempool.reset(), gotten)
    }

    #[test]
    fn test_mempool_dupes() {
        let mut mempool = MemPool::new(10);
        let r = Record::new("hello".to_string());
        mempool.put(r.clone()).unwrap();
        assert_eq!(mempool.length(), 1);
        assert!(mempool.contains(&r));
        mempool.put(r.clone()).unwrap();
        assert_eq!(mempool.length(), 1);
        assert!(mempool.contains(&r));
        mempool.reset();
        assert_eq!(mempool.length(), 0);
    }
}