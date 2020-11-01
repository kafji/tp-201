use slog::{info, o, Drain, Logger};
use std::{
    env,
    io::{BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
};

const ADDRESS: &str = "127.0.0.1:6379";

fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = Logger::root(drain, o!());

    info!(log, "redis protocol");

    let opt: String = env::args().skip(1).take(1).collect();
    match opt.as_str() {
        "server" => {
            let log = log.new(o!());
            server::start(log)
        }
        "client" => {
            let log = log.new(o!());
            client::start(log)
        }
        other => panic!(format!("unexpected arg `{}`", other)),
    }
}

fn buffered(stream: TcpStream) -> (impl BufRead, impl Write) {
    let r_stream = stream.try_clone().expect("failed to clone stream");
    let reader = BufReader::new(r_stream);
    let writer = BufWriter::new(stream);
    (reader, writer)
}

mod server {
    use super::*;
    use std::{io::BufRead, net::TcpListener};

    #[derive(Eq, PartialEq, Debug)]
    enum Request {
        Ping(Option<String>),
    }

    impl Request {
        fn parse(text: &str) -> Self {
            if !text.starts_with("PING") {
                panic!("expected `PING`, found `{}`", text);
            }
            let (arg_start, _) = text
                .char_indices()
                .nth(5)
                .expect("expected request length is at least 5 characters");
            let arg_end = text.chars().count() - 2;
            let has_arg = arg_end > arg_start;
            if has_arg {
                let arg = &text[arg_start..arg_end];
                Request::Ping(Some(arg.to_owned()))
            } else {
                Request::Ping(None)
            }
        }
    }

    #[cfg(test)]
    #[test]
    fn test_request_parse() {
        let request = Request::parse("PING\r\n");
        assert_eq!(request, Request::Ping(None));

        let request = Request::parse("PING hello\r\n");
        assert_eq!(request, Request::Ping(Some("hello".to_owned())));
    }

    pub fn start(log: Logger) {
        info!(log, "server");
        let listener = TcpListener::bind(ADDRESS)
            .unwrap_or_else(|_| panic!("failed to bind to address: {}", ADDRESS));

        loop {
            info!(log, "waiting for incoming connection");
            let (stream, socket_addr) = listener
                .accept()
                .expect("expected successful tcp connection");
            info!(log, "stream: {:?}", stream);
            info!(log, "socket addr: {:?}", socket_addr);

            let mut close_stream = false;
            let (mut reader, mut writer) = buffered(stream);

            loop {
                let request = read_request(&log, &mut reader);
                info!(log, "request: {:?}", request);

                match request {
                    Some(Request::Ping(arg)) => respond_ping(&log, &mut writer, arg),
                    None => close_stream = true,
                };

                // Flush write buffer before taking another request or closing stream.
                writer.flush().expect("failed to flush write buffer");

                if close_stream {
                    info!(log, "closing stream");
                    break;
                }
            }
        }
    }

    fn read_request(log: &Logger, reader: &mut impl BufRead) -> Option<Request> {
        info!(log, "reading request");

        let mut request = String::new();

        // XXX: `read_line` will block until it found `\n`.
        let size = reader
            .read_line(&mut request)
            .expect("expected request in utf-8");

        if size == 0 {
            info!(log, "reached EOF while reading request");
            return None;
        }

        info!(log, "literal request: {:?}", request);

        let request = Request::parse(&request);
        Some(request)
    }

    fn respond_ping(log: &Logger, stream: &mut impl Write, arg: Option<String>) {
        info!(log, "sending ping response");

        let response = match arg {
            Some(arg) => format!("+PONG {}\r\n", arg),
            None => "+PONG\r\n".to_owned(),
        };

        info!(log, "response: {:?}", response);

        stream
            .write_all(response.as_bytes())
            .expect("failed to write response");
    }
}

mod client {
    use super::*;
    use std::{io::BufRead, io::Write, net::TcpStream, thread, time::Duration};

    pub fn start(log: Logger) {
        info!(log, "client");

        info!(log, "conecting to server");
        let stream = TcpStream::connect(ADDRESS)
            .unwrap_or_else(|_| panic!("failed to connect to address: {}", ADDRESS));
        info!(log, "stream: {:?}", stream);

        let (mut reader, mut writer) = buffered(stream);

        info!(log, "sleeping for 1 sec before sending request");
        thread::sleep(Duration::from_secs(1));

        request_ping(&log, &mut writer, None);
        let response = read_response(&log, &mut reader);
        info!(log, "response: {:?}", response);

        request_ping(&log, &mut writer, Some("hello".to_owned()));
        let response = read_response(&log, &mut reader);
        info!(log, "response: {:?}", response);

        info!(log, "closing stream");
    }

    fn request_ping(log: &Logger, writer: &mut impl Write, arg: Option<String>) {
        info!(log, "sending ping request");

        let request = match arg {
            Some(arg) => format!("PING {}\r\n", arg),
            None => "PING\r\n".to_owned(),
        };

        info!(log, "request: {:?}", request);

        writer
            .write_all(request.as_bytes())
            .expect("failed to write request");

        // Send request immediately.
        writer.flush().expect("failed to flush write buffer");
    }

    fn read_response(log: &Logger, reader: &mut impl BufRead) -> String {
        info!(log, "reading response");

        let mut response = String::new();

        // XXX: `read_line` will block until it found `\n`.
        let size = reader
            .read_line(&mut response)
            .expect("failed to read response");

        info!(log, "read: {}", size);

        response
    }
}
