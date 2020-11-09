use slog::{debug, error, Logger};

use crate::{
    protocol::{Request, Response},
    KvsEngine, KvsEngineError,
};

pub trait HandleRequest {
    type Error;

    fn handle(&mut self, log: &Logger, request: Request) -> Result<Response, Self::Error>;
}

// I love how composable Rust is.
impl<T> HandleRequest for T
where
    T: KvsEngine + ?Sized,
{
    type Error = anyhow::Error;

    fn handle(&mut self, log: &Logger, request: Request) -> Result<Response, Self::Error> {
        match request {
            Request::Set { key, value } => {
                let result = self.set(key, value);
                let response = match result {
                    Ok(_) => Response::Success(None),
                    Err(e) => Response::Failure(e.to_string()),
                };
                Ok(response)
            }
            Request::Get { key } => {
                debug!(log, "GET request");
                let result = self.get(key);
                let response = match result {
                    Ok(v) => Response::Success(v),
                    Err(e) => Response::Failure(e.to_string()),
                };
                Ok(response)
            }
            Request::Rm { key } => {
                let result = self.remove(&key);
                let response = match result {
                    Ok(_) => {
                        debug!(log, "entry removed"; "key" => key);
                        Response::Success(None)
                    }
                    Err(KvsEngineError::EntryNotFound { .. }) => {
                        debug!(log, "entry not found"; "key" => key);
                        Response::Failure("Key not found".to_owned())
                    }
                    Err(e) => {
                        error!(log, "error on removing entry"; "error" => ?e, "key" => key);
                        Response::Failure(e.to_string())
                    }
                };
                Ok(response)
            }
        }
    }
}
