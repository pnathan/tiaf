use crate::chain::{Blockchain};
use crate::record::{Record};
use crate::woody::Attributes;

#[macro_use]
use crate::woody;
#[macro_use]
use crate::notes;
use crate::mempool::MemPool;
use crate::query_chain;

use rouille::{Request, Response};
use std::ops::Deref;
use query_chain::Queryable;
use crate::api;
use crate::peers::{Downstreams, ReadHost, Upstreams};
use std::sync::{Arc, RwLock};
use crate::api::{TiafBoringResponse, TiafDownstreams, TiafUpstreams, AdminKey};

fn auth(request: &Request, admin_key: AdminKey) -> Result<(), Response> {
    request.header("X-TIAF-ADMIN-KEY")
        .and_then(|k| {
            if admin_key.eq_str(k) {
                Some(())
            } else {
                None
            }
        }).ok_or_else(|| {
        rouille::Response::json(&TiafBoringResponse::Error("invalid admin key".to_string()))
            .with_status_code(401)
    })
}

pub fn launch_server(
    node_id: String,
    ip: String,
    port: u16,
    admin_key: AdminKey,
    blockchain: Arc<RwLock<Blockchain>>,
    mem_pool: Arc<RwLock<MemPool>>,
    downstreams: Arc<RwLock<Downstreams>>,
    upstreams: Arc<RwLock<Upstreams>>,
) {
    let logger = woody::new(woody::Level::Info);

    let endpoint = format!("{}:{}", ip, port);
    logger.info(notes!(
        "ts",
        chrono::Utc::now().to_rfc3339(),
        "msg",
        "starting http server".to_string(),
        "endpoint",
        endpoint.clone()
    ));

    rouille::start_server(endpoint, move |request: &Request| {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let ts = chrono::Utc::now();

        let mut result = router!(request,
            (GET) (/) => {
                rouille::Response::text("index")
            },

            (GET) (/healthz) => {
                rouille::Response::text("OK")
            },

            (GET) (/api/v1/chain) => {
                let b = blockchain.read().unwrap();
                rouille::Response::json(b.deref())
            },

            (GET) (/api/v1/chain/tail/{n: u64}) => {
                let b= blockchain.read().unwrap();
                let blocks = b.tail(n);
                let nublocks = blocks.iter()
                .map(|b| (**b).clone()).collect();
                let response = api::TiafPartialChain{
                    partial_blocks: nublocks,
                    total_length: b.length(),
                };
                rouille::Response::json(&response)
            },

            (GET) (/api/v1/chain/since/{hash: String}) => {
                let b = blockchain.read().unwrap();
                let blocks = b.since(&hash);
                match blocks {
                    Ok(blocks) => {
                        let nublocks = blocks.iter()
                        .map(|b| (**b).clone()).collect();
                        let response = api::TiafPartialChain{
                            partial_blocks: nublocks,
                            total_length: b.length()
                        };
                        rouille::Response::json(&response)
                    },
                    Err(e) => {
                        logger.error(notes!("ts", chrono::Utc::now().to_rfc3339(), "error", e.to_string()));
                        rouille::Response::json(&TiafBoringResponse::Error(e)).with_status_code(500)
                    }
                }
            },

            (POST) (/api/v1/chain/compare) => {
                let body: Blockchain = try_or_400!(rouille::input::json_input(&request));
                let b = blockchain.read().unwrap();
                let result = b.compare_other_chain(&body);

                logger.info(notes!("result", format!("{:?}", result)));
                rouille::Response::json(&api::TiafCompareResult{ result })
            },
            (OPTIONS) (/api/v1/chain/compare) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },

            (GET) (/api/v1/statistics) => {
                let b = blockchain.read().unwrap();
                let mp = mem_pool.read().unwrap();
                rouille::Response::json(
                    &api::TiafStatistics{
                        node_id: node_id.clone(),
                        chain_length: b.length(),
                        pool_size: mp.length() as u64,
                        downstream_count: downstreams.read().unwrap().deref().downstreams().len() as u64,
                        upstream_count: upstreams.read().unwrap().deref().upstreams().len() as u64,
                })
            },

            // this is the conventional place to write rows to the data table
            (POST) (/api/v1/data) => {
                let body: api::RecordPut = try_or_400!(rouille::input::json_input(&request));

                let mut mp = mem_pool.write().unwrap();
                let r = Record::new(body.data);
                _ = mp.put(r);
                // log the write
                logger.info(notes!("ts", chrono::Utc::now().to_rfc3339(), "msg", "data added to mempool".to_string()));

                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (OPTIONS) (/api/v1/data) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },

            // the record endpoint is used for sharing new records between peers.
             (POST) (/api/v1/record) => {
                let r: Record = try_or_400!(rouille::input::json_input(&request));
                let mut mp = mem_pool.write().unwrap();
                _ = mp.put(r);
                // log the write
                logger.info(notes!("ts", chrono::Utc::now().to_rfc3339(), "msg", "record added to mempool".to_string()));

                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (OPTIONS) (/api/v1/record) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (GET) (/api/v1/query) => {

                match request.get_param("q")  {
                    Some(q) =>  {
                        let clozed_query = query_chain::Query::new(q).unwrap().parse();

                        let b = blockchain.read().unwrap();

                        match b.query(clozed_query) {
                            Ok(result) => rouille::Response::json(&result),
                            Err(e) => {
                                logger.error(notes!("ts", chrono::Utc::now().to_rfc3339(), "error", e.to_string()));
                                return rouille::Response::json(&TiafBoringResponse::Error(e))
                                .with_status_code(400);
                            }
                        }
                    }
                    None => {
                        rouille::Response::json(&TiafBoringResponse::Error("missing query parameter".to_string()))
                        .with_status_code(400)
                    }

                }
            },
            (GET) (/api/v1/admin/upstream) => {
                if let Err(x) = auth(request, admin_key.clone())  {
                    return x;
                }

                let response = upstreams.read().unwrap();
                rouille::Response::json(&response.to_api())
            },
            (POST) (/api/v1/admin/upstream) => {
                if let Err(x) = auth(request, admin_key.clone())  {
                    return x;
                }
                let r: TiafUpstreams = try_or_400!(rouille::input::json_input(&request));
                // convert TiafUpstream to Upstreams
                let mut input = upstreams.write().unwrap();
                *input = Upstreams::from_api(&r);
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (OPTIONS) (/api/v1/admin/upstream) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (POST) (/api/v1/admin/upstream/toggle) => {
                if let Err(x) = auth(request, admin_key.clone())  {
                    return x;
                }
                let mut input = upstreams.write().unwrap();
                input.sweeping = ! input.sweeping;
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (OPTIONS) (/api/v1/admin/upstream/enable) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },


            (GET) (/api/v1/admin/downstream) => {
                let response = downstreams.read().unwrap();
                rouille::Response::json(&response.to_api())
            },
            (POST) (/api/v1/admin/downstream) => {
                if let Err(x) = auth(request, admin_key.clone())  {
                    return x;
                }
                let r: TiafDownstreams = try_or_400!(rouille::input::json_input(&request));
                // convert TiafDownstream to Downstreams
                let mut input = downstreams.write().unwrap();
                *input = Downstreams::from_api(&r);
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (OPTIONS) (/api/v1/admin/downstream) => {
                rouille::Response::json(&TiafBoringResponse::Ok)
            },
            (POST) (api/v1/admin/downstream/toggle) => {
                if let Err(x) = auth(request, admin_key.clone())  {
                    return x;
                }
                // convert TiafDownstream to Downstreams
                let mut input = downstreams.write().unwrap();
                input.sweeping = ! input.sweeping;
                rouille::Response::json(&TiafBoringResponse::Ok)
            },

            (GET) (/api/v1/admin/node-id) => {
                let response = api::TiafNode{
                    node_id: node_id.clone()
                };
                rouille::Response::json(&response)
            },

            _ => rouille::Response::empty_404());

        let code = result.status_code;
        logger.info(vec![
            Attributes::KV("ts", ts.to_rfc3339()),
            Attributes::KV("duration", format!("{:?}", start.elapsed()).to_string()),
            Attributes::KV("method", request.method().to_string()),
            Attributes::KV("url", request.url()),
            Attributes::KV("code", code.to_string())]);

        result.headers.push(("Access-Control-Allow-Origin".into(), "*".into()));
        result.headers.push(("Access-Control-Allow-Headers".into(), "Origin, X-Requested-With, Content-Type, Accept, authorization".into()));
        result.headers.push(("Access-Control-Allow-Methods".into(), "GET, POST, OPTIONS, PUT, DELETE".into()));

        result
    });
}
