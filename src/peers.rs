// TODO: rename file to network or something beyond just peer
use crate::api::{TiafClient, TiafDownstreams, TiafPartialChain, TiafUpstreams};
use crate::chain::Blockchain;
use crate::mempool::MemPool;
use crate::record::Record;
use crate::{notes, woody, Attributes};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct WriteHost {
    url: String,
    last_pushed: Option<std::time::Instant>,
    latest_hash: Option<crate::types::Hashtype>,
}

impl WriteHost {
    pub fn new(s: &String) -> WriteHost {
        WriteHost {
            url: s.to_string(),
            last_pushed: None,
            latest_hash: None,
        }
    }
    pub fn url(&self) -> String {
        self.url.clone()
    }
    pub fn notify_host(&mut self, r: &Record) -> Result<(), String> {
        // admin key set to false. Peers are not admins.
        let client = TiafClient::new(self.url.clone(), None);
        client.put_record(r)?;
        self.latest_hash = Some(r.hash.clone());
        self.last_pushed = Some(std::time::Instant::now());
        Ok(())
    }
}

#[derive(Clone)]
pub struct Downstreams {
    hosts: Vec<WriteHost>,
    pub sweeping: bool,
}

impl Downstreams {
    pub fn to_api(&self) -> TiafDownstreams {
        let hosts = self.hosts.iter().map(|h| h.url.clone()).collect();
        TiafDownstreams {
            hosts,
            sweeping: self.sweeping,
        }
    }

    pub fn from_api(api: &TiafDownstreams) -> Downstreams {
        let hosts = api.hosts.iter().map(|h| WriteHost::new(h)).collect();
        Downstreams {
            hosts,
            sweeping: api.sweeping,
        }
    }

    pub fn new(p: Vec<WriteHost>) -> Downstreams {
        Downstreams {
            hosts: p,
            sweeping: false,
        }
    }
    pub fn downstreams(&self) -> Vec<WriteHost> {
        return self.hosts.iter().map(|p| p.clone()).collect();
    }
}

#[derive(Clone)]
pub struct ReadHost {
    pub url: String,
    // None implies never swept.
    last_swept: Option<std::time::Instant>,
    latest_hash: Option<crate::types::Hashtype>,
}

impl ReadHost {
    pub fn new(s: &String) -> ReadHost {
        ReadHost {
            url: s.to_string(),
            last_swept: None,
            latest_hash: None,
        }
    }
}

pub struct Upstreams {
    hosts: Vec<ReadHost>,
    pub sweeping: bool,
}

impl Upstreams {
    pub fn to_api(&self) -> TiafUpstreams {
        let hosts = self.hosts.iter().map(|h| h.url.clone()).collect();
        TiafUpstreams {
            hosts,
            sweeping: self.sweeping,
        }
    }

    pub fn from_api(api: &TiafUpstreams) -> Upstreams {
        let hosts = api.hosts.iter().map(|h| ReadHost::new(h)).collect();
        Upstreams {
            hosts,
            sweeping: api.sweeping,
        }
    }

    pub fn new(hosts: Vec<ReadHost>) -> Upstreams {
        Upstreams {
            hosts,
            sweeping: false,
        }
    }

    pub fn upstreams(&self) -> Vec<ReadHost> {
        return self.hosts.iter().map(|p| p.clone()).collect();
    }

    pub fn set_sweeping(&mut self, sweeping: bool) {
        self.sweeping = sweeping;
    }

    pub fn add(&mut self, peer: ReadHost) {
        self.hosts.push(peer);
    }

    pub fn remove(&mut self, peer: &ReadHost) {
        self.hosts.retain(|p| p.url != peer.url);
    }

    fn get_upstream(url: &str) -> Result<TiafPartialChain, String> {
        match reqwest::blocking::get(url.clone()) {
            Ok(resp) => {
                // TODO: work out the proper API for this one.
                match resp.json::<TiafPartialChain>() {
                    Ok(chain) => Ok(chain),
                    Err(e) => Err(format!("failed to parse json: {}", e)),
                }
            }
            Err(e) => Err(format!("failed to get peer: {}", e)),
        }
    }

    /// sweep_all_peers will sweep all upstreams and update the chain if a longer chain is found.
    /// This does not relate to the mempool.
    pub fn sweep_all_upstreams(&mut self, chain: &mut Blockchain) -> Result<(), String> {
        let logger = woody::new(woody::Level::Info);
        for host in &mut self.hosts {
            // get blocks of hashes from peer and compare list of hashes to existing chain.
            // if longer, request blocks from peer to glom on starting from the hash that wasn't seen.
            /// TODO: work out proper api for this one.
            let url = host.url.clone() + "/api/v1/chain";
            let other_chain = Upstreams::get_upstream(url.as_str())?;
            if other_chain.total_length > chain.length() {
                let starting_idx = other_chain
                    .partial_blocks
                    .iter()
                    .position(|b| !chain.block_seen(&b.hash));

                match starting_idx {
                    Some(idx) => {
                        logger.info(notes!(
                            "msg",
                            format!("found starting index: {}", idx).to_string()
                        ));
                        chain.append_blocks(other_chain.partial_blocks[idx..].to_vec())?;
                    }
                    None => {
                        logger.warn(notes!("msg", "no starting index found".to_string()));
                    }
                }
                if starting_idx.is_none() {
                    logger.warn(notes!("msg", "no starting index found".to_string()));
                    continue;
                }
            }
            host.latest_hash = Some(
                other_chain.partial_blocks[other_chain.partial_blocks.len() - 1]
                    .hash
                    .clone(),
            );
            host.last_swept = Some(std::time::Instant::now());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_add_remove_upstreams() {
        let mut upstreams = super::Upstreams::new(vec![]);
        let host = super::ReadHost::new(&"http://localhost:8080".to_string());
        upstreams.add(host.clone());
        assert_eq!(upstreams.hosts.len(), 1);
        upstreams.remove(&host);
        assert_eq!(upstreams.hosts.len(), 0);
    }
}
