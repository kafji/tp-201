use clap::Clap;
use kvs::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clap)]
#[clap(version=VERSION)]
struct Opts {
    #[clap()]
    addr: String,
    #[clap()]
    engine: String,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    Ok(())
}
