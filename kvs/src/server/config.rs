use std::{fmt, net, str};

pub struct Configuration {
    pub addr: net::SocketAddr,
    pub engine: Engine,
}

#[derive(Debug)]
pub enum Engine {
    KVS,
}

impl str::FromStr for Engine {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let engine = match s.to_lowercase().as_ref() {
            "kvs" => Engine::KVS,
            other => return Err(format!("unknown engine `{}`", other).into()),
        };
        Ok(engine)
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Engine::KVS => write!(f, "kvs"),
        }
    }
}
