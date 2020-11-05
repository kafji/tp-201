use clap::Clap;
use kvs::app::logger;
use kvs::KvsServer;
use kvs::Result;
use kvs::*;
use slog::{info, o};

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap(long, default_value = DEFAULT_ADDR)]
    addr: String,
    #[clap(long, default_value = DEFAULT_ENGINE)]
    engine: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let addr: std::net::SocketAddr = opts.addr.parse().unwrap();
    let engine: kvs::server::Engine = opts.engine.parse().unwrap();

    let log = slog::Logger::root(
        logger::drain(),
        o! { "version" => VERSION, "address" => addr, "engine" => format!("{}", engine) },
    );

    info!(log, "starting");

    let config = kvs::server::Configuration { addr, engine };
    let mut server = KvsServer::new(log, config);

    server.listen()?;

    Ok(())
}
