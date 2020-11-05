use clap::Clap;
use kvs::KvsServer;
use kvs::Result;
use slog::{info, o, Drain};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_ADDR: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: &str = "kvs";

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap(long, default_value = DEFAULT_ADDR)]
    addr: String,
    #[clap(long, default_value = DEFAULT_ENGINE)]
    engine: String,
}

fn main() -> Result<()> {
    let decorator = slog_term::TermDecorator::new().stderr().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let opts: Opts = Opts::parse();
    let addr = opts.addr;
    let engine = opts.engine;

    let log = slog::Logger::root(
        drain,
        o! { "version" => VERSION, "address" => addr, "engine" => engine },
    );

    info!(log, "starting server");

    let server = KvsServer::new(log);

    Ok(())
}
