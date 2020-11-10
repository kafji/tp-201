use crate::{
    protocol::{Request, Response},
    KvsEngine, KvsEngineError,
};
use slog::{debug, error, Logger};

pub trait HandleRequest {
    fn handle(&mut self, log: &Logger, request: Request) -> Result<Response, KvsEngineError>;
}

// I love how composable Rust is.
impl<T> HandleRequest for T
where
    T: KvsEngine + ?Sized,
{
    fn handle(&mut self, log: &Logger, request: Request) -> Result<Response, KvsEngineError> {
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
                let result = self.get(&key);
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
