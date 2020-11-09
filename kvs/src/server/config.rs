use std::{fmt, str};

#[derive(Clone, Debug)]
pub enum EngineOpt {
    KVS,
}

impl Default for EngineOpt {
    fn default() -> Self {
        EngineOpt::KVS
    }
}

impl str::FromStr for EngineOpt {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "kvs" => Ok(EngineOpt::KVS),
            other => Err(format!("unknown engine `{}`", other).into()),
        }
    }
}

impl fmt::Display for EngineOpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineOpt::KVS => write!(f, "kvs"),
        }
    }
}
