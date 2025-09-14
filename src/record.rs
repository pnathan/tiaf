use crate::hexdisplay::HexDisplayExt;
use crate::types::{Hashtype, Time};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;
use unicode_bidi::BidiInfo;
use uuid::Uuid;

// Record is a standalone entry into the data datable.
// It is worth noting that Entry might need to be rewritten to u8.
// As it stands, the likely type of the entry is JSON. Should that be normative?
// A Record can be considered sort of a row in a database table. But one can _batch_ them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    // This is unique per record
    pub uuid: Uuid,
    // This is, ideally, unique per record.
    pub timestamp: Time,
    // This is whatever.
    pub entry: String,
    // This is the hash of the above.
    pub hash: Hashtype,
}

// A KVRecord is a list of key-value pairs: a row in a database table, as it were.
// It is indelibly associated with its source Record, as represented by the lifetime association.
// However, the JSON parsing out of the entry field is not done will generate new string allocations,
// as represented by the 'pairs' being new strings.
pub struct KVRecord<'a> {
    pub source: &'a Record,
    pairs: HashMap<String, String>,
}

impl<'a> KVRecord<'a> {
    pub fn pairs(&self) -> HashMap<String, String> {
        self.pairs.clone()
    }
}

impl Hash for Record {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /*   self.uuid.hash(state);
        self.timestamp.hash(state);
        self.entry.hash(state);*/
        self.hash.hash(state);
    }
}

impl fmt::Display for Record {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let entry = self.entry.to_string();
        let entry = niqqud::remove_thorough(&entry).to_string();
        let bidi_info = BidiInfo::new(&entry, None);
        let para = &bidi_info.paragraphs[0];
        let line = para.range.clone();
        let display = bidi_info.reorder_line(para, line);

        let hash = self.hash.clone();

        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "uuid: {}, timestamp: {}, entry: {}, hash: {}",
            self.uuid, self.timestamp, display, hash
        )
    }
}

#[allow(dead_code)]
enum RecordErrors {
    Deserialization(String),
}

impl Record {
    pub fn genesis_record() -> Record {
        let bereshit = "בְּרֵאשִׁ֖ית בָּרָ֣א".to_string();
        let mut r = Record {
            uuid: Default::default(),
            timestamp: 0,
            entry: bereshit,
            hash: "rec-init".to_string(),
        };
        r.ensure_hash();
        r
    }

    // New generates a fully hashed record with a proper timestamp.
    pub fn new(data: String) -> Record {
        let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };
        let mut r = Record {
            uuid: Uuid::new_v4(),
            timestamp: now,
            entry: data,
            hash: "rec-init".to_string(),
        };
        // a valid Record always has a valid hash.
        r.ensure_hash();
        r
    }
    fn ensure_hash(&mut self) {
        if self.hash == "rec-init" {
            let time_bytes = self.timestamp.to_be_bytes();
            let entry_bytes = self.entry.as_bytes();
            let uuid_bytes = self.uuid.as_bytes().to_vec();
            let mut bytes = Vec::from(time_bytes);
            bytes.extend(entry_bytes.to_vec().iter());
            bytes.extend(uuid_bytes.iter());

            let mut hasher = Sha3_256::new();
            hasher.update(bytes);

            self.hash = hasher.finalize().to_vec().hex_display().to_string();
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let time_bytes = self.timestamp.to_be_bytes();
        let entry_bytes = self.entry.as_bytes();
        let uuid_bytes = self.uuid.as_bytes().to_vec();
        let mut bytes = Vec::from(time_bytes);
        bytes.extend(entry_bytes.to_vec().iter());
        bytes.extend(uuid_bytes.iter());

        let mut hasher = Sha3_256::new();
        hasher.update(bytes);
        let hash = hasher.finalize().to_vec().hex_display().to_string();
        if hash == self.hash {
            Ok(())
        } else {
            Err(format!("record hash mismatch: {} != {}", hash, self.hash))
        }
    }
    pub fn structured_entry(&self) -> Result<KVRecord, String> {
        let j = serde_json::from_str(&self.entry).map_err(|e| e.to_string())?;

        Ok(KVRecord {
            source: self,
            pairs: j,
        })
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Record {}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn test_records() {
        let g = Record::genesis_record();
        let r = Record::new("sixpence".to_string());
        _ = serde_json::to_string(&g).unwrap();
        _ = serde_json::to_string(&r).unwrap();
    }

    #[test]
    fn test_structured_entry() {
        let r = Record::new("{\"foo\": \"bar\"}".to_string());
        let kv = r.structured_entry().unwrap();
        assert_eq!(kv.pairs.get("foo").unwrap(), "bar");
    }
    #[test]
    fn test_structured_entry_with_unstructure() {
        let r = Record::new("mumbles".to_string());
        assert!(r.structured_entry().is_err());
    }
}
