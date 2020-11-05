use clap::Clap;
use kvs::{app::logger, client::KvsClient, *};
use kvs::{KvStore, Result};
use slog::{info, o};
use std::{net::SocketAddr, path::Path, process::exit};

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap(long, default_value = DEFAULT_ADDR)]
    addr: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Get(Get),
    Set(Set),
    Rm(Rm),
    #[clap(about = "Show all entries")]
    List,
}

#[derive(Clap)]
#[clap(about = "Get value of a key")]
struct Get {
    #[clap(about = "Entry key")]
    key: String,
}

#[derive(Clap)]
#[clap(about = "Set value for a key")]
struct Set {
    #[clap(about = "Entry key")]
    key: String,
    #[clap(about = "Entry value")]
    value: String,
}

#[derive(Clap)]
#[clap(about = "Remove entry")]
struct Rm {
    #[clap(about = "Entry key")]
    key: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let addr: SocketAddr = opts.addr.parse().unwrap();
    let log = slog::Logger::root(
        logger::drain(),
        o! { "version" => VERSION, "address" => addr },
    );
    info!(log, "starting");
    let config = kvs::client::Configuration {
        addr: opts.addr.parse().unwrap(),
    };
    let client = KvsClient::new(log, config);
    client.connect()?;

    if true {
        return Ok(());
    }

    let mut store = KvStore::open(Path::new("./"))?;

    let opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Get(get) => match store.get(get.key)? {
            Some(value) => {
                println!("{}", value);
                Ok(())
            }
            None => {
                println!("Key not found");
                Ok(())
            }
        },
        SubCommand::Set(set) => store.set(set.key, set.value),
        SubCommand::Rm(rm) => match store.remove(rm.key) {
            Ok(_) => Ok(()),
            Err(error) => match error {
                kvs::Error::KeyNotFound => {
                    println!("Key not found");
                    exit(1);
                }
                _ => Err(error),
            },
        },
        SubCommand::List => {
            let entries = store.list()?;
            for (key, value) in entries {
                println!("{} -> {}", key, value);
            }
            Ok(())
        }
    }
}
