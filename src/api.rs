use crate::block::Block;
use crate::chain::{Blockchain, ChainComparison};
use crate::record::Record;
use serde::{Deserialize, Serialize};
use url::Url;

// API admin key type, methods, etc
#[derive(Debug, Clone)]
pub struct AdminKey(String);
impl AdminKey {
    pub fn new(s: &str) -> AdminKey {
        AdminKey(s.to_string())
    }
    pub fn get(&self) -> String {
        self.0.clone()
    }

    pub fn eq_str(&self, s: &str) -> bool {
        self.0 == s
    }

    pub fn eq_string(&self, s: &String) -> bool {
        self.0 == *s
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TiafBoringResponse {
    Ok,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TiafNode {
    pub node_id: String,
}

// This might need a new name
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordPut {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TiafStatistics {
    pub node_id: String,
    pub chain_length: u64,
    pub pool_size: u64,
    pub downstream_count: u64,
    pub upstream_count: u64,
}

/// TiafPartialChain is an API interface struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct TiafPartialChain {
    pub total_length: u64,
    pub partial_blocks: Vec<Block>,
}

// This might need to be reworked
#[derive(Debug, Serialize, Deserialize)]
pub struct TiafCompareResult {
    pub result: ChainComparison,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TiafUpstreams {
    pub hosts: Vec<String>,
    pub sweeping: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TiafDownstreams {
    pub hosts: Vec<String>,
    pub sweeping: bool,
}

// API Client Code
pub struct TiafClient {
    url: Url,
    #[allow(dead_code)]
    admin_key: Option<AdminKey>,
}

impl TiafClient {
    pub fn new(url: String, key: Option<String>) -> TiafClient {
        TiafClient {
            url: Url::parse(&url).unwrap(),
            admin_key: key.map(|key| AdminKey::new(&key)),
        }
    }
    pub fn get_full_chain(&self) -> Result<Blockchain, String> {
        let url = match self.url.clone().join("/api/v1/chain") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::get(url) {
            Ok(resp) => match resp.json::<Blockchain>() {
                Ok(chain) => Ok(chain),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }
    pub fn get_chain_tail(&self, n: u64) -> Result<TiafPartialChain, String> {
        let url = match self
            .url
            .clone()
            .join(format!("/api/v1/chain/tail/{n}").as_str())
        {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::get(url) {
            Ok(resp) => match resp.json::<TiafPartialChain>() {
                Ok(chain) => Ok(chain),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }
    pub fn get_chain_since(&self, hash: &String) -> Result<TiafPartialChain, String> {
        let url = match self
            .url
            .clone()
            .join(format!("/api/v1/chain/since/{hash}").as_str())
        {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::get(url) {
            Ok(resp) => match resp.json::<TiafPartialChain>() {
                Ok(chain) => Ok(chain),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }
    pub fn post_compare(&self, chain: &Blockchain) -> Result<TiafCompareResult, String> {
        let url = match self.url.clone().join("/api/v1/chain/compare") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::Client::new()
            .post(url)
            .json(chain)
            .send()
        {
            Ok(resp) => match resp.json::<TiafCompareResult>() {
                Ok(chain) => Ok(chain),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }
    pub fn get_statistics(&self) -> Result<TiafStatistics, String> {
        let url = match self.url.clone().join("/api/v1/statistics") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::get(url) {
            Ok(resp) => match resp.json::<TiafStatistics>() {
                Ok(chain) => Ok(chain),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }

    pub fn put_data(&self, records: &RecordPut) -> Result<(), String> {
        let url = match self.url.clone().join("/api/v1/data") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::Client::new()
            .put(url)
            .json(records)
            .send()
        {
            Ok(resp) => match resp.status() {
                reqwest::StatusCode::OK => Ok(()),
                _ => Err(format!("failed to put data: {}", resp.status())),
            },
            Err(e) => Err(format!("failed to put data: {e}")),
        }
    }

    pub fn put_record(&self, record: &Record) -> Result<(), String> {
        let url = match self.url.clone().join("/api/v1/record") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };
        match reqwest::blocking::Client::new()
            .put(url)
            .json(record)
            .send()
        {
            Ok(resp) => match resp.status() {
                reqwest::StatusCode::OK => Ok(()),
                _ => Err(format!("failed to put record: {}", resp.status())),
            },
            Err(e) => Err(format!("failed to put record: {e}")),
        }
    }

    pub fn query(&self, query: String) -> Result<Vec<Record>, String> {
        let mut url = match self.url.clone().join("/api/v1/query/") {
            Ok(url) => url,
            Err(e) => return Err(format!("failed to form url: {e}")),
        };

        url.set_query(Some(format!("q={}", &query).as_str()));

        match reqwest::blocking::get(url) {
            Ok(resp) => match resp.json::<Vec<Record>>() {
                Ok(records) => Ok(records),
                Err(e) => Err(format!("failed to parse json: {e}")),
            },
            Err(e) => Err(format!("failed to get chain: {e}")),
        }
    }
}
