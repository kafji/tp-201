use clap::Clap;
use kvs::{KvStore, VERSION};
use std::{path::Path, process::exit};

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = KvStore::open(Path::new("./"))?;

    let opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Get(get) => match store.get(get.key)? {
            Some(value) => {
                println!("{}", value);
            }
            None => {
                println!("Key not found");
            }
        },
        SubCommand::Set(set) => store.set(set.key, set.value)?,
        SubCommand::Rm(rm) => match store.remove(rm.key) {
            Ok(_) => {}
            Err(error) => match error {
                kvs::store::KvStoreError::KeyNotFound { .. } => {
                    println!("Key not found");
                    exit(1);
                }
                _ => Err(error)?,
            },
        },
        SubCommand::List => {
            let entries = store.list()?;
            for (key, value) in entries {
                println!("{} -> {}", key, value);
            }
        }
    }

    Ok(())
}
