use super::HandleRequest;
use crate::{
    protocol::{Request, Response, Serialization, SerializationError},
    KvsEngineError,
};
use nix::{
    sys::{
        epoll::{epoll_create, epoll_ctl, epoll_wait, EpollEvent, EpollFlags, EpollOp},
        eventfd::*,
    },
    unistd,
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use slog::{debug, error, info, o, Discard, Logger};
use std::{
    io,
    net::{SocketAddr, TcpListener},
    os::unix::io::{AsRawFd, RawFd},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("failed to bind socket, caused by {0}")]
    BindSocketError(io::Error),
    #[error("failed to accept connection, caused by {0}")]
    AcceptConnectionError(io::Error),
    #[error(transparent)]
    Sys(#[from] nix::Error),
    #[error(transparent)]
    Engine(#[from] KvsEngineError),
    #[error("failed to write response, caused by {0}")]
    ResponseError(#[from] SerializationError),
}

#[derive(FromPrimitive, Debug)]
enum PollId {
    Listener,
    Signal,
}

pub struct KvsServer {
    log: Logger,
    listener: TcpListener,
    signal_fd: RawFd,
}

impl Drop for KvsServer {
    fn drop(&mut self) {
        unistd::close(self.signal_fd).expect("failed to close signal file descriptor");
    }
}

impl KvsServer {
    pub fn new(
        log: impl Into<Option<Logger>>,
        address: impl Into<SocketAddr>,
    ) -> Result<Self, ServerError> {
        let log = log.into().unwrap_or_else(|| Logger::root(Discard, o!()));

        debug!(log, "binding TCP listener");
        let addr = address.into();
        let listener = {
            fn error_mapper(error: io::Error) -> ServerError {
                ServerError::BindSocketError(error)
            }
            let listener = TcpListener::bind(addr).map_err(error_mapper)?;
            // Blocking for request mechanism is handled by epoll.
            listener.set_nonblocking(true).map_err(error_mapper)?;
            listener
        };

        debug!(log, "creating signal eventfd");
        let signal_fd = eventfd(0, EfdFlags::empty())?;

        let server = Self {
            log,
            listener,
            signal_fd,
        };
        Ok(server)
    }

    pub fn listen(&self, handler: &mut impl HandleRequest) -> Result<(), ServerError> {
        // Alias self.log so it's easier to cascade logger.
        let log = &self.log;

        info!(log, "start listening");

        debug!(log, "epoll create");
        let epfd = epoll_create()?;

        let mut signal_ev = EpollEvent::new(
            {
                let mut flags = EpollFlags::empty();
                flags.set(EpollFlags::EPOLLIN, true);
                flags
            },
            PollId::Signal as _,
        );
        epoll_ctl(epfd, EpollOp::EpollCtlAdd, self.signal_fd, &mut signal_ev)?;

        let mut listener_ev = EpollEvent::new(
            {
                let mut flags = EpollFlags::empty();
                flags.set(EpollFlags::EPOLLIN, true);
                flags
            },
            PollId::Listener as _,
        );
        epoll_ctl(
            epfd,
            EpollOp::EpollCtlAdd,
            self.listener.as_raw_fd(),
            &mut listener_ev,
        )?;

        let mut shutdown = false;
        loop {
            const EPOLL_MAXEVENTS: usize = 2;
            const EPOLL_TIMEOUT: isize = -1;

            let log = log.new(o!(
                "epoll_maxevents" => EPOLL_MAXEVENTS,
                "epoll_timeout" => EPOLL_TIMEOUT
            ));

            let mut events = [EpollEvent::empty(); EPOLL_MAXEVENTS];

            info!(log, "waiting for incoming connection");

            debug!(log, "epoll wait");
            let count = epoll_wait(epfd, &mut events, EPOLL_TIMEOUT)?;

            for event in events.iter().take(count) {
                match PollId::from_u64(event.data()) {
                    Some(PollId::Listener) => {
                        debug!(log, "incoming connection received");
                        let (mut stream, peer) = self
                            .listener
                            .accept()
                            .map_err(|x| ServerError::AcceptConnectionError(x))?;
                        let log = log.new(o!("peer" => peer));

                        info!(log, "connected");
                        let mut quit = false;
                        loop {
                            let request = Request::from_reader(&mut stream);

                            match request {
                                Ok(Some(request)) => {
                                    info!(log, "received request"; "request" => ?request);
                                    let response = handler.handle(&log, request)?;
                                    info!(log, "sending response"; "response" => ?response);
                                    response.to_writer(&mut stream)?;
                                }
                                Ok(None) => {
                                    debug!(log, "received eof");
                                    quit = true;
                                }
                                Err(err) => {
                                    error!(log, "received invalid request"; "error" => %err);
                                    let response = Response::Failure("invalid request".to_owned());
                                    info!(log, "sending response"; "response" => ?response);
                                    response.to_writer(&mut stream)?;
                                }
                            };

                            if quit {
                                info!(log, "closing connection");
                                break;
                            }
                        }
                    }
                    Some(PollId::Signal) => {
                        debug!(log, "shutdown signal receieved");
                        shutdown = true;
                    }
                    None => unimplemented!(),
                }
            }
            if shutdown {
                info!(log, "shutting down");
                break;
            }
        }
        Ok(())
    }

    /// Set signal to shutdown the server.
    ///
    /// Probably only used in testing. Required to cleanup test.
    #[cfg(test)]
    fn shutdown(&self) -> Result<(), ServerError> {
        unistd::write(self.signal_fd, &1u64.to_ne_bytes())?;
        Ok(())
    }

    /// Returns address where server is bound to.
    ///
    /// Probably only used in testing. Helpful when server address port is set to zero.
    #[cfg(test)]
    fn address(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        protocol::{Request, Response},
        KvStore,
    };
    use slog::{o, Discard};
    use std::{net::TcpStream, sync::Arc, thread::spawn, time::Duration};

    #[test]
    fn test_can_be_shutdown() {
        let server = {
            let log = slog::Logger::root(Discard, o!());
            let address = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
            let server = KvsServer::new(log, address).unwrap();
            Arc::new(server)
        };

        let handle = {
            let server = server.clone();
            spawn(move || {
                let dir = tempfile::tempdir().unwrap().into_path();
                let mut engine = KvStore::open(&dir).unwrap();
                server.listen(&mut engine).unwrap();
            })
        };

        // Shutdown.
        server.shutdown().unwrap();

        handle.join().unwrap();
    }

    macro_rules! request {
        ($writer:expr, $request:expr) => {
            $request.to_writer(&mut $writer).unwrap();
        };
    }

    macro_rules! response {
        ($reader:expr, $response:expr) => {
            assert_eq!(
                Some($response),
                Response::from_reader(&mut $reader).unwrap()
            );
        };
    }

    /// Basic request <-> response tests.
    ///
    /// Doesn't cover all possible requests and responses.
    #[test]
    fn test_handle_requests() {
        let server = {
            let log = Logger::root(Discard, o!());
            let address = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
            let server = KvsServer::new(log, address).unwrap();
            Arc::new(server)
        };

        let handle = {
            let server = server.clone();
            spawn(move || {
                let dir = tempfile::tempdir().unwrap().into_path();
                let mut engine = KvStore::open(&dir).unwrap();
                server.listen(&mut engine).unwrap();
            })
        };

        let mut client = {
            let address = server.address().unwrap();
            TcpStream::connect_timeout(&address, Duration::from_millis(100)).unwrap()
        };

        request!(
            client,
            Request::Set {
                key: "key1".to_owned(),
                value: "value1".to_owned(),
            }
        );
        response!(client, Response::Success(None));

        request!(
            client,
            Request::Get {
                key: "key1".to_owned(),
            }
        );
        response!(client, Response::Success(Some("value1".to_owned())));

        // Disconnect.
        drop(client);

        server.shutdown().unwrap();
        handle.join().unwrap();
    }
}
