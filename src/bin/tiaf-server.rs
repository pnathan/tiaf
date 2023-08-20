use tiaf;

use clap::Parser;

use rand::Rng;
use std::thread;
use std::time::Duration;

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tiaf::api::AdminKey;
use tiaf::chain::Blockchain;
use tiaf::mempool::MemPool;
use tiaf::peers::{Downstreams, ReadHost, Upstreams, WriteHost};
use tiaf::record::Record;
use tiaf::woody::{Level, Logger};
use tiaf::{notes, woody, Attributes};

fn sleep_with_jitter(d: Duration, width: u32) {
    let jitter = rand::random::<u64>() % (width * 1000) as u64;
    // d +/- jitter/2.
    let d = d - Duration::from_millis(jitter) / 2 + Duration::from_millis(jitter);
    thread::sleep(d);
}

/// sleep with 10% jitter to avoid thundering herd.
fn sleep_with_10p_jitter(d: Duration) {
    sleep_with_jitter(d, d.as_secs() as u32 / 10);
}

struct DownstreamsNotifier {
    mp: Arc<RwLock<MemPool>>,
    downstreams: Arc<RwLock<Downstreams>>,
    sleep_duration: Duration,
}

// or do we have a fifo queue that flushes to network?
fn downstream_notifying_daemon(args: DownstreamsNotifier) {
    let logger = woody::new(woody::Level::Info);
    loop {
        downstream_notify(&args, &logger);
        logger.info(notes!(
            "ts",
            chrono::Utc::now().to_rfc3339(),
            "msg",
            "sleeping downstream notifier".to_string()
        ));
        sleep_with_10p_jitter(args.sleep_duration);
    }
}

fn downstream_notify(args: &DownstreamsNotifier, logger: &Logger) {
    let mp = args.mp.read().unwrap();
    let hosts = args.downstreams.read().unwrap();
    if hosts.sweeping {
        let records: Vec<&Record> = mp.contents().iter().collect();
        for mut p in hosts.downstreams() {
            for r in records.iter() {
                match p.notify_host(r) {
                    Ok(_) => {
                        logger.info(notes!(
                            "ts",
                            chrono::Utc::now().to_rfc3339(),
                            "msg",
                            format!("notified downstream of record: {}", p.url()).to_string()
                        ));
                    }
                    Err(e) => {
                        logger.error(notes!(
                            "ts",
                            chrono::Utc::now().to_rfc3339(),
                            "msg",
                            format!("failed to notify downstream of record: {}", e).to_string()
                        ));
                    }
                }
            }
        }
    } else {
        logger.info(notes!(
            "ts",
            chrono::Utc::now().to_rfc3339(),
            "msg",
            "downstream notification disabled".to_string()
        ));
    }
}

struct PoolSweeper {
    chain: Arc<RwLock<Blockchain>>,
    mp: Arc<RwLock<MemPool>>,
    // sleep_duration- everyone has the same duration.
    sleep_duration: Duration,
}

fn pool_sweeping_daemon(args: PoolSweeper) {
    let logger = woody::new(woody::Level::Info);
    loop {
        {
            pool_sweep(&args, &logger);
        }
        logger.info(notes!(
            "ts",
            chrono::Utc::now().to_rfc3339(),
            "msg",
            "sleeping mempool sweeper".to_string()
        ));
        sleep_with_10p_jitter(args.sleep_duration);
    }
}

fn pool_sweep(args: &PoolSweeper, logger: &Logger) {
    let mut mp = args.mp.write().unwrap();
    if mp.length() > 0 {
        let mut p = args.chain.write().unwrap();
        let records: Vec<Record> = mp.reset().drain().collect();

        match p.append_new_records(records) {
            Ok(_) => {
                logger.info(notes!(
                    "ts",
                    chrono::Utc::now().to_rfc3339(),
                    "msg",
                    "appended records to chain".to_string()
                ));
            }
            Err(e) => {
                logger.error(notes!(
                    "ts",
                    chrono::Utc::now().to_rfc3339(),
                    "msg",
                    format!("failed to append records to chain: {}", e).to_string()
                ));
            }
        }
    }
}

struct UpstreamSweeper {
    chain: Arc<RwLock<Blockchain>>,
    sleep_duration: Duration,
    upstreams: Arc<RwLock<tiaf::peers::Upstreams>>,
}

/// upstream_sweeping_daemon is a daemon that periodically sweeps all peers in the Peers
/// struct. It will attempt to update the blockchain
fn upstream_sweeping_daemon(args: UpstreamSweeper) {
    let logger = woody::new(woody::Level::Info);
    loop {
        upstream_sweep(&args, &logger);
        logger.info(notes!(
            "ts",
            chrono::Utc::now().to_rfc3339(),
            "msg",
            "sleeping upstream sweeper".to_string()
        ));
        sleep_with_10p_jitter(args.sleep_duration);
    }
}

fn upstream_sweep(args: &UpstreamSweeper, logger: &Logger) {
    let mut chain = args.chain.write().unwrap();
    let mut hosts = args.upstreams.write().unwrap();
    if hosts.sweeping {
        match hosts.sweep_all_upstreams(&mut chain) {
            Ok(_) => {
                logger.info(notes!(
                    "ts",
                    chrono::Utc::now().to_rfc3339(),
                    "msg",
                    "swept all upstreams".to_string()
                ));
            }
            Err(e) => {
                logger.error(notes!(
                    "ts",
                    chrono::Utc::now().to_rfc3339(),
                    "msg",
                    format!("failed to sweep upstreams: {}", e).to_string()
                ));
            }
        }
    } else {
        logger.info(notes!(
            "ts",
            chrono::Utc::now().to_rfc3339(),
            "msg",
            "upstream sweeping disabled".to_string()
        ));
    }
}

/// ServerConfig is also used as the schema for the config file.
#[derive(Debug, Serialize, Deserialize)]
struct ServerConfig {
    #[serde(default)]
    node_id: String,

    ip: String,
    #[serde(default)]
    downstreams: Vec<String>,
    #[serde(default)]
    upstreams: Vec<String>,
    port: u16,
    #[serde(default)]
    log_level: Level,
}

#[derive(Debug, Parser)]
#[command(name = "tiaf")]
#[command(author = "pnathan <paul@nathan.house>")]
#[command(version = "0.2")]
#[command(about = "blockchain with trust and without currency", long_about = None)]
struct Cli {
    /// Node ID. Can be random, so long as unique.
    /// If not set, will be generated.
    #[arg(long, required = false)]
    node_id: Option<String>,

    /// IP to bind to
    #[arg(long, required = false)]
    ip: Option<String>,
    /// Port for tiaf server
    #[arg(long, required = false)]
    port: Option<u16>,
    /// Servers to push to, but not read from.
    #[arg(long, required = false)]
    downstreams: Option<Vec<String>>,
    /// Servers to read from, but not push to
    #[arg(long, required = false)]
    upstreams: Option<Vec<String>>,
    /// File for configuration in TOML format
    #[arg(long, required = false)]
    config: Option<std::path::PathBuf>,
    /// Logging level
    #[arg(long, required = false)]
    log_level: Option<String>,
}

fn parse_arguments() -> Result<ServerConfig, String> {
    let cli = Cli::parse();

    let rand_string: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let mut server_config: ServerConfig = ServerConfig {
        ip: "127.0.0.1".to_string(),
        downstreams: vec![],
        upstreams: vec![],
        port: 9999,
        node_id: rand_string,
        log_level: woody::Level::Info,
    };
    if let Some(config) = cli.config {
        let config = match std::fs::read_to_string(config) {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("failed to read config file: {}", e));
            }
        };
        server_config = match toml::from_str(&config) {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("failed to parse config file: {}", e));
            }
        }
    }

    if let Some(ip) = cli.ip {
        server_config.ip = ip;
    }

    if let Some(port) = cli.port {
        server_config.port = port;
    }

    if let Some(downstreams) = cli.downstreams {
        server_config.downstreams = downstreams;
    }
    if let Some(upstreams) = cli.upstreams {
        server_config.upstreams = upstreams;
    }

    if let Some(log_level) = cli.log_level {
        server_config.log_level = Level::from_string(log_level.as_str())?;
    }
    println!("{:?}", server_config);
    return Ok(server_config);
}

#[derive(Clone)]
struct ServerGlobals {
    blockchain: Arc<RwLock<Blockchain>>,
    mem_pool: Arc<RwLock<MemPool>>,
    upstreams: Arc<RwLock<Upstreams>>,
    downstreams: Arc<RwLock<Downstreams>>,
}

fn main() {
    let logger = woody::new(woody::Level::Info);

    let server_config = parse_arguments().unwrap();
    logger.debug(notes!("server_config", format!("{:?}", server_config)));

    let sg = ServerGlobals {
        blockchain: Arc::new(RwLock::new(Blockchain::new())),
        mem_pool: Arc::new(RwLock::new(MemPool::new(8))),
        upstreams: Arc::new(RwLock::new(Upstreams::new(
            server_config.upstreams.iter().map(ReadHost::new).collect(),
        ))),
        downstreams: Arc::new(RwLock::new(Downstreams::new(
            server_config
                .downstreams
                .iter()
                .map(WriteHost::new)
                .collect(),
        ))),
    };

    let pool_sweeper_sg = sg.clone();
    // start the mempool sweeper
    thread::spawn(move || {
        pool_sweeping_daemon(PoolSweeper {
            chain: pool_sweeper_sg.blockchain,
            mp: pool_sweeper_sg.mem_pool,
            sleep_duration: Duration::from_secs(15),
        })
    });

    let chain_sweeper_sg = sg.clone();
    thread::spawn(move || {
        upstream_sweeping_daemon(UpstreamSweeper {
            chain: chain_sweeper_sg.blockchain,
            sleep_duration: Duration::from_secs(15),
            upstreams: chain_sweeper_sg.upstreams,
        })
    });

    let peer_sg = sg.clone();
    thread::spawn(move || {
        downstream_notifying_daemon(DownstreamsNotifier {
            mp: peer_sg.mem_pool,
            sleep_duration: Duration::from_secs(15),
            downstreams: peer_sg.downstreams,
        })
    });

    let http_sg = sg.clone();

    logger.info(notes!(
        "server",
        "launching server".to_string(),
        "port",
        server_config.port.to_string(),
        "ip",
        server_config.ip.clone()
    ));
    tiaf::server::launch_server(
        server_config.node_id,
        server_config.ip,
        server_config.port,
        AdminKey::new("bob"),
        http_sg.blockchain,
        http_sg.mem_pool,
        http_sg.downstreams,
        http_sg.upstreams,
    );
}

#[cfg(test)]
mod toplevel {
    use super::*;

    #[test]
    fn exercise() {
        let r = Record::genesis_record();
        let r2 = Record::new("sixes".to_string());
        let r3 = Record::new("sevens".to_string());
        let r4 = Record::new("eights".to_string());
        let rs = vec![r2, r3, r4];
        let r2 = Record::new("foo".to_string());
        let r3 = Record::new("bar ".to_string());
        let r4 = Record::new("baz".to_string());
        let rs2 = vec![r2, r3, r4];
        let bc = Arc::new(RwLock::new(Blockchain::new()));
        {
            let mut b = bc.write().unwrap();
            _ = b.append_records(rs.to_vec());
            _ = b.append_records(rs2.to_vec());
            let j = b.to_json(true).unwrap();
            let bb = Blockchain::from_json(j).unwrap();
        }
    }
}
