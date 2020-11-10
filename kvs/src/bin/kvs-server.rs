use clap::Clap;
use kvs::{app::logger, KvsServer, DEFAULT_ADDR, DEFAULT_ENGINE, VERSION};
use slog::{info, o};
use std::{error, fmt, net::SocketAddr};

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap(long, default_value = DEFAULT_ADDR)]
    addr: String,
    #[clap(long, default_value = DEFAULT_ENGINE)]
    engine: String,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let log = slog::Logger::root(logger::drain(), o!());

    let opts: Opts = Opts::parse();

    let address: SocketAddr = opts
        .addr
        .parse()
        .map_err(|_| format!("failed to parse addr `{}`", opts.addr))?;

    let engine_opt: Engine = opts.engine.parse().map_err(|_| {
        format!(
            "failed to parse engine, expected `kvs` or `sled`, found `{}`",
            opts.engine
        )
    })?;

    info!(log, "starting"; "address" => address, "engine" => %engine_opt);

    let mut engine: Box<dyn kvs::KvsEngine> = match engine_opt {
        Engine::KVS => Box::new(kvs::KvStore::open("./")?),
        Engine::Sled => Box::new(sled::open("./")?),
    };
    let server = KvsServer::new(log, address)?;
    server.listen(&mut engine)?;

    Ok(())
}

#[derive(Clone, Debug)]
pub enum Engine {
    KVS,
    Sled,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::KVS
    }
}

impl std::str::FromStr for Engine {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let engine = match s.to_lowercase().as_ref() {
            "kvs" => Engine::KVS,
            "sled" => Engine::Sled,
            other => return Err(format!("unknown engine `{}`", other).into()),
        };
        Ok(engine)
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Engine::KVS => write!(f, "kvs"),
            Engine::Sled => write!(f, "sled"),
        }
    }
}
