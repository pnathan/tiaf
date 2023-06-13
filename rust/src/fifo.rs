use crate::fifo::FifoError::{Empty, Full, Illogical};

#[derive(Debug, PartialEq)]
pub enum FifoError {
    Full,
    Empty,
    Illogical,
}

// Fifo will be used as the key mempool structure, the prelim buffer that will be swept
// into the Blockchain periodically.
#[derive(Debug)]
pub struct Fifo<T> where T: Clone {
    records: Vec<Option<T>>,
    reader: usize,
    writer: usize,
    length: usize,
    max_size: usize,
}


impl<T> Fifo<T> where T: Clone {

    // Drain all records out, resetting the queue. This is a destructive operation.
    pub fn drain(&mut self) -> Vec<T> {
        let mut v: Vec<T> = Vec::new();
        for i in 0..self.length {
            let r = self.records[(self.reader + i) % self.max_size].clone();
            v.push(r.unwrap());
        }

        let r = vec![None; self.max_size];
        self.records = r;
        self.reader = 0;
        self.writer = 0;
        self.length = 0;
        v
    }

    pub fn new(max_size: usize) -> Fifo<T> {
        let r = vec![None; max_size];
        Fifo {
            records: r,
            reader: 0,
            writer: 0,
            length: 0,
            max_size,
        }
    }

    pub fn put(&mut self, r: T) -> Result<(), FifoError> {
        if self.length == self.max_size {
            return Err(Full);
        }

        self.records[self.writer] = Some(r);
        self.writer = (self.writer + 1) % self.max_size;
        self.length += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<T, FifoError> {
        if self.length == 0 {
            return Err(Empty);
        }
        let r = self.records[self.reader].clone();
        self.reader = (self.reader + 1) % self.max_size;
        self.length -= 1;
        r.ok_or(Illogical)
    }

    pub fn length(&self) -> usize {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Record;
    use super::*;

    #[test]
    fn test_fifo() {
        let mut fifo = Fifo::new(10);
        let r = Record::new("test".to_string());
        fifo.put(r.clone()).unwrap();
        assert_eq!(fifo.length(), 1);
        let r2 = fifo.pop().unwrap();
        assert_eq!(r, r2);
        assert_eq!(fifo.length(), 0);
    }

    #[test]
    fn test_fifo_full() {
        let mut fifo = Fifo::<Record>::new(10);
        for i in 0..10 {
            let r = Record::new(i.to_string());
            fifo.put(r).unwrap();
        }
        assert_eq!(fifo.length(), 10);
        let r = Record::new("test".to_string());
        assert_eq!(fifo.put(r), Err(FifoError::Full));
    }

    #[test]
    fn test_fifo_empty() {
        let mut fifo = Fifo::<Record>::new(10);
        assert_eq!(fifo.length(), 0);
        assert_eq!(fifo.pop(), Err(FifoError::Empty));
    }

    #[test]
    fn test_fifo_max_top() {
        let mut fifo = Fifo::new(10);
        let mut recs = Vec::<Record>::new();
        for i in 0..10 {
            let r = Record::new(i.to_string());
            recs.push(r.clone());
            fifo.put(r).unwrap();
        }
        assert_eq!(fifo.length(), 10);
        assert_eq!(fifo.pop(), Ok(recs[0].clone()));
    }

    #[test]
    fn test_fifo_drain() {
        let mut fifo = Fifo::new(10);
        let mut recs = Vec::<Record>::new();
        for i in 0..10 {
            let r = Record::new(i.to_string());
            recs.push(r.clone());
            fifo.put(r).unwrap();
        }
        assert_eq!(fifo.length(), 10);
        let v = fifo.drain();
        assert_eq!(v.len(), 10);
        assert_eq!(fifo.length(), 0);
        assert_eq!(v, recs);
    }
}