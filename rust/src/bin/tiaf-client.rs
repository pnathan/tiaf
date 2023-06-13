use std::{fmt, fs};
use std::str::FromStr;
use clap::{Arg, ArgAction, Command, Parser};
use tiaf::chain::{Blockchain};
use tiaf::woody;
use tiaf::api;

#[derive(Clone, Debug)]
struct TiafArgs {
    host: String,
    port: u16,
    log_level: u8,
    admin_key: Option<String>
}

impl fmt::Display for TiafArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TiafClient {{ ip: {}, port: {}, log_level: {} }}", self.host, self.port, self.log_level)
    }
}

impl TiafArgs {
    fn client(&self) -> api::TiafClient {
        api::TiafClient::new(format!("{}:{}", self.host, self.port), self.admin_key.clone())
    }
}

fn main() {
    let command_structure = Command::new("tiaf")
        .arg_required_else_help(true)
        .arg(
            Arg::new("loglevel")
                .short('v')
                .long("loglevel")
                .default_value("1")
                .help("set log level - 0-3 (least to most), default 1"))
        .arg(
            Arg::new("host")
                .short('u')
                .long("host")
                .required(true)
                .help("specify a tiaf host"))
        .arg(Arg::new("port")
            .short('p')
            .long("port")
            .required(true)
            .help("specify a tiaf port"))
        .arg(Arg::new("adminKey")
            .short('k')
            .long("adminKey")
            .required(false)
            .help("specify a tiaf admin key"))
        .subcommand(
            Command::new("query").short_flag('Q')
                .arg(
                Arg::new("query")
                    .long("query")
                    .short('q')
                    .exclusive(true)
                    .help("query the tiaf chain"))
                .arg(Arg::new("file")
                    .long("file")
                    .exclusive(true)
                    .help("query the tiaf chain from a file")))
        .subcommand(
            Command::new("statistics").short_flag('S'))
        .subcommand(Command::new("chain").short_flag('C')
            .subcommand(Command::new("full-read").short_flag('F')
                .about("read the full chain"))
            .subcommand(Command::new("partial-read").short_flag('P')
                .about("read the partial chain"))
            .arg(
                Arg::new("hash")
                    .short('a')
                    .long("hash")
                    .help("read from hash H").exclusive(true))
            .arg(
                Arg::new("tail")
                    .short('n')
                    .long("number")
                    .help("read the last n blocks").exclusive(true)));


    let matches = command_structure.get_matches();

    let global_args: TiafArgs = TiafArgs {
        host: matches.get_one::<String>("host").unwrap().clone(),
        port: u16::from_str(&matches.get_one::<String>("port").unwrap()).unwrap(),
        log_level: u8::from_str(&matches.get_one::<String>("loglevel").unwrap()).unwrap(),
        admin_key: matches.get_one::<String>("adminKey").map(|s| s.to_string())
    };
    let logger = woody::new(woody::Level::from_u8(&global_args.log_level).unwrap());

    println!("args: {}", global_args);

    if let Some(sub_m) = matches.subcommand_matches("chain") {
        if let Some(sub_m) = sub_m.subcommand_matches("full-read") {

            let s: Result<Blockchain, String> = global_args.client().get_full_chain();
            match s {
                Ok(s) => println!("{:?}", s),
                Err(e) => println!("chain: error: {}", e),
            }
        }
        if let Some(sub_m) = sub_m.subcommand_matches("partial-read") {
            println!("chain: partial-read");
        }
        if let Some(sub_m) = sub_m.get_one::<String>("hash") {
            println!("chain: hash {}", sub_m);
        }
        if let Some(sub_m) = sub_m.get_one::<String>("tail") {
            println!("chain: tail {}", sub_m);
        }
    }

    if let Some(sub_m) = matches.subcommand_matches("statistics") {
        let s: Result<api::TiafStatistics, String> = global_args.client().get_statistics();
        match  s {
            Ok(s) => println!("{:?}", s),
            Err(e) => println!("statistics: error: {}", e),
        }
    }

    if let Some(sub_m) = matches.subcommand_matches("query") {
        let inline = sub_m.get_one::<String>("query");
        let file = sub_m.get_one::<String>("file");
        if inline.is_none() && file.is_none() {
            println!("query: no query specified");
            return;
        }
        if inline.is_some() && file.is_some() {
            println!("query: both inline and file specified");
            return;
        }

        let query = inline
            .map(|s| s.to_string())
            .or_else(|| {
                match fs::read_to_string(&file.unwrap()) {
                    Ok(s) => Some(s.trim().to_string()),
                    Err(e) => {
                        panic!("query: error reading file: {}", e);
                    }
                }
            }).unwrap();
        println!("search: {}", query);
        println!("{:?}", global_args.client().query(query));
    }
}