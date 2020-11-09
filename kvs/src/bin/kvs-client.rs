use clap::Clap;
use kvs::KvStore;
use kvs::{app::logger, client::KvsClient, DEFAULT_ADDR, VERSION};
use slog::{info, o};
use std::{error, net::SocketAddr, path::Path, process::exit};

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap(long, default_value = DEFAULT_ADDR, global = true)]
    addr: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Get(Get),
    Set(Set),
    Rm(Rm),
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log = slog::Logger::root(slog::Discard, o!("version" => VERSION));

    let opts: Opts = Opts::parse();

    let address = opts
        .addr
        .parse::<SocketAddr>()
        .map_err(|_| format!("failed to parse addr `{}`", opts.addr))?;

    info!(log, "starting"; "address" => address);

    let mut client = KvsClient::new(log, address)?;

    match opts.subcmd {
        SubCommand::Get(Get { key }) => {
            let v = client.get(key)?;
            match v {
                Some(v) => println!("{}", v),
                None => println!("Key not found"),
            }
        }
        SubCommand::Set(Set { key, value }) => {
            client.set(key, value)?;
        }
        SubCommand::Rm(Rm { key }) => {
            client.rm(key)?;
        }
    }

    Ok(())
}
