use clap::Clap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

fn main() {
    let opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Get(_) => {
            panic!("unimplemented");
        }
        SubCommand::Set(_) => {
            panic!("unimplemented");
        }
        SubCommand::Rm(_) => {
            panic!("unimplemented");
        }
    }
}
