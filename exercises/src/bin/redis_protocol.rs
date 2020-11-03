// Hookay. This file is a mess.
//
// I think using serde is a mistake. Because it makes things more complicated. serde is great if we
// just want to map from serialized format into lang type. But in this case our serialized format
// has attribute attached to it.
//
// From high level perspective, RESP has 3 string-ish types, simple string, error, and bulk string.
// Where should we deserialize those types into? If we deserialize all of those into
// `std::string::String` that makes RESP error type transparent and we lose error handling.
//
// The other option is to serialize those into user declared enum.
// ```
// enum RespString {
//     Simple, Error, Bulk
// }
// ```
// Now we can check if response is just a string or an error string. But that introduce different
// problem. Since the enum variant to deserialize to is described internally (not by parsing the
// input) the name for the enum variants must be as the deserializer expects.
//
// Rather than that. I think it would be better to have the data structures and a reader that will // yields those defined in the protocol module.
// ```
// // Callsite where client reading server response.
// let input = impl Read;
// let ty = input.read_type();
// match ty {
//     Type::Error(msg) => panic!("{}, oh no!", msg),
//     Type::SimpleString(txt) if txt == "OK" => println!("success!"),
// }
// ```

use slog::{info, o, Drain, Logger};
use std::{
    env,
    io::{BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
};

const ADDRESS_IP: &str = "127.0.0.1";
const DEFAULT_ADDRESS_PORT: u16 = 6379;

const CRLF: &[u8] = b"\r\n";

fn main() {
    let log = root_logger();

    info!(log, "redis protocol");

    let opt: String = env::args().skip(1).take(1).collect();
    match opt.as_str() {
        "server" => {
            let log = log.new(o!());
            server::start(log, DEFAULT_ADDRESS_PORT)
        }
        "client" => {
            let log = log.new(o!());
            client::start(log, DEFAULT_ADDRESS_PORT)
        }
        other => panic!(format!("unexpected arg `{}`", other)),
    }
}

fn root_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}

#[cfg(test)]
fn test_logger() -> Logger {
    let noop = true;
    if noop {
        Logger::root(slog::Discard, o!())
    } else {
        root_logger()
    }
}

fn buffered(stream: TcpStream) -> (impl BufRead, impl Write) {
    let r_stream = stream.try_clone().expect("failed to clone stream");
    let reader = BufReader::new(r_stream);
    let writer = BufWriter::new(stream);
    (reader, writer)
}

mod server {
    use super::{protocol::*, *};
    use std::net::TcpListener;

    #[cfg(test)]
    use std::{io::Read, thread, time::Duration};

    pub fn start(log: Logger, port: u16) {
        info!(log, "server");
        let addr = (ADDRESS_IP, port);
        let listener = TcpListener::bind(addr)
            .unwrap_or_else(|_| panic!("failed to bind to address: {:?}", addr));

        let mut shutdown = false;

        loop {
            info!(log, "waiting for incoming connection");
            let (stream, socket_addr) = listener
                .accept()
                .expect("expected successful tcp connection");
            info!(log, "stream: {:?}", stream);
            info!(log, "socket addr: {:?}", socket_addr);

            let mut close = false;
            let (mut reader, mut writer) = buffered(stream);

            loop {
                let request = Request::from_reader(&mut reader);

                info!(log, "request: {:?}", request);

                match request {
                    Request::Ping { arg } => respond_ping(&log, &mut writer, arg),
                    Request::Shutdown => {
                        shutdown = true;
                        close = true;
                    } // _ => close = true,
                };

                // Flush write buffer before taking another request or closing stream.
                writer.flush().expect("failed to flush write buffer");

                if close {
                    info!(log, "closing stream");
                    break;
                }
            }

            if shutdown {
                info!(log, "shutting down");
                break;
            }
        }
    }

    #[cfg(test)]
    #[test]
    fn test_server_shutdown() {
        let log = test_logger();

        let port = {
            // XXX: Works as long as the returned port doesn't get reused by OS.
            // Better solution is to bind the listener on different thread and send the port through
            // channel for the client to connect to.
            let stream = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = stream.local_addr().unwrap();
            addr.port()
        };

        let server_handle = thread::spawn(move || {
            start(log, port);
        });

        // Wait for server to start.
        // Again, better solution is to send signal through channel.
        thread::sleep(Duration::from_millis(100));

        let addr = ("127.0.0.1", port);
        let mut conn = TcpStream::connect(addr).unwrap();
        conn.write_all(b"*1\r\n$8\r\nSHUTDOWN\r\n").unwrap();
        // serialize::to_writer(&mut conn, &vec!["SHUTDOWN".to_owned()]).unwrap();

        // Assert server thread is finished.
        server_handle.join().unwrap();
    }

    #[cfg(test)]
    #[test]
    fn test_server_ping_without_argument() {
        let log = test_logger();

        let port = {
            // XXX: Works as long as the returned port doesn't get reused by OS.
            // Better solution is to bind the listener on different thread and send the port through
            // channel for the client to connect to.
            let stream = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = stream.local_addr().unwrap();
            addr.port()
        };

        let server_handle = thread::spawn(move || {
            start(log, port);
        });

        // Wait for server to start.
        // Again, better solution is to send signal through channel.
        thread::sleep(Duration::from_millis(100));

        let addr = ("127.0.0.1", port);
        let mut conn = TcpStream::connect(addr).unwrap();
        conn.write_all(b"*1\r\n$4\r\nPING\r\n").unwrap();

        let mut response = [0; 7];
        conn.read_exact(&mut response).unwrap();
        assert_eq!(b"+PONG\r\n", &response);

        // Shutdown server.
        conn.write_all(b"*1\r\n$8\r\nSHUTDOWN\r\n").unwrap();
        server_handle.join().unwrap();
    }

    #[cfg(test)]
    #[test]
    fn test_server_ping_with_argument() {
        let log = test_logger();

        let port = {
            // XXX: Works as long as the returned port doesn't get reused by OS.
            // Better solution is to bind the listener on different thread and send the port through
            // channel for the client to connect to.
            let stream = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = stream.local_addr().unwrap();
            addr.port()
        };

        let server_handle = thread::spawn(move || {
            start(log, port);
        });

        // Wait for server to start.
        // Again, better solution is to send signal through channel.
        thread::sleep(Duration::from_millis(100));

        let addr = ("127.0.0.1", port);
        let mut conn = TcpStream::connect(addr).unwrap();
        conn.write_all(b"*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n")
            .unwrap();

        let mut response = [0; 11];
        conn.read_exact(&mut response).unwrap();
        assert_eq!(b"$5\r\nhello\r\n", &response);

        // Shutdown server.
        conn.write_all(b"*1\r\n$8\r\nSHUTDOWN\r\n").unwrap();
        server_handle.join().unwrap();
    }

    fn respond_ping(log: &Logger, stream: &mut impl Write, arg: Option<String>) {
        let log = log.new(o! { "arg" => format!("{:?}", arg) });
        info!(log, "sending ping response");
        let value = match arg {
            Some(arg) => protocol::Type::BulkString(arg),
            None => protocol::Type::SimpleString("PONG".to_owned()),
        };
        serialize::to_writer(stream, &value).unwrap();
    }

    #[cfg(test)]
    #[test]
    fn test_respond_ping_without_arg() {
        let log = test_logger();
        let mut output = Vec::<u8>::new();

        respond_ping(&log, &mut output, None);

        let output = String::from_utf8(output).unwrap();
        assert_eq!("+PONG\r\n", output);
    }

    #[cfg(test)]
    #[test]
    fn test_respond_ping_with_arg() {
        let log = test_logger();
        let mut output = Vec::<u8>::new();

        respond_ping(&log, &mut output, Some("hello".to_owned()));

        let output = String::from_utf8(output).unwrap();
        assert_eq!("$5\r\nhello\r\n", output);
    }
}

mod client {
    use super::*;
    use crate::protocol::Request;
    use std::{io::BufRead, io::Write, net::TcpStream, thread, time::Duration};

    pub fn start(log: Logger, port: u16) {
        info!(log, "client");

        info!(log, "conecting to server");
        let addr = (ADDRESS_IP, port);
        let stream = TcpStream::connect(addr)
            .unwrap_or_else(|_| panic!("failed to connect to address: {:?}", addr));
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

        let flag = true;
        if flag {
            let request = protocol::Request::Ping { arg };
            send_request(log, writer, request);
            return;
        }

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

    fn send_request(log: &Logger, writer: &mut impl Write, request: Request) {
        let log = log.new(o! { "request"=> format!("{:?}", request) });
        info!(log, "sending request");
        request.to_writer(writer).expect("failed to write request");
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

    #[cfg(test)]
    #[test]
    fn test_request_ping_without_arg() {
        let log = test_logger();
        let mut output = Vec::<u8>::new();

        request_ping(&log, &mut output, None);

        let output = String::from_utf8(output).unwrap();
        assert_eq!("PING\r\n", output);
    }

    #[cfg(test)]
    #[test]
    fn test_request_ping_with_arg() {
        let log = test_logger();
        let mut output = Vec::<u8>::new();

        request_ping(&log, &mut output, Some("hello".to_owned()));

        let output = String::from_utf8(output).unwrap();
        assert_eq!("PING hello\r\n", output);
    }
}

mod protocol {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::{self, Read, Write};

    /// RESP types.
    #[derive(Deserialize, Serialize, Eq, PartialEq, Debug)]
    pub enum Type {
        SimpleString(String),
        Error(String),
        Integer(i32),
        BulkString(String),
        Array(Vec<Type>),
    }

    #[derive(Deserialize, Serialize, Eq, PartialEq, Debug)]
    pub enum Request {
        Ping { arg: Option<String> },
        Shutdown,
    }

    impl Request {
        pub fn from_reader(reader: &mut impl Read) -> Self {
            let request: Vec<String> = deserialize::from_reader(reader).unwrap();
            let mut request = request.into_iter();
            let command = request.next().map(|x| x.to_uppercase());
            match command {
                Some(cmd) if cmd == "PING" => {
                    let arg = request.next();
                    Request::Ping { arg }
                }
                Some(cmd) if cmd == "SHUTDOWN" => Request::Shutdown,
                Some(cmd) => panic!("unknown command `{}`", cmd),
                None => panic!("expected command"),
            }
        }

        pub fn to_writer(&self, writer: &mut impl Write) -> Result<(), io::Error> {
            match self {
                Request::Ping { arg } => {
                    writer.write_all(b"PING")?;
                    if let Some(arg) = arg {
                        writer.write_all(b" ")?;
                        writer.write_all(arg.as_bytes())?;
                    }
                    writer.write_all(CRLF)?;
                }
                _ => todo!(),
            }
            Ok(())
        }
    }
}

mod serialize {
    use super::*;
    use serde::Serialize;
    use std::{error, fmt};

    #[derive(Debug)]
    pub enum Error {
        ExpectedKnownLength,
    }

    impl fmt::Display for Error {
        fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
            todo!()
        }
    }

    impl error::Error for Error {}

    impl serde::ser::Error for Error {
        fn custom<T>(_msg: T) -> Self
        where
            T: fmt::Display,
        {
            todo!()
        }
    }

    impl From<std::io::Error> for Error {
        fn from(_: std::io::Error) -> Self {
            todo!()
        }
    }

    pub struct Serializer<W>
    where
        W: Write,
    {
        writer: W,
        ty: Option<TypeIdent>,
    }

    #[derive(Debug)]
    enum TypeIdent {
        SimpleString,
        Error,
        BulkString,
    }

    pub fn to_writer<T>(writer: &mut impl Write, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer { writer, ty: None };
        value.serialize(&mut serializer)?;
        Ok(())
    }

    impl<'a, W> serde::ser::Serializer for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;
        type SerializeSeq = Self;
        type SerializeTuple = Self;
        type SerializeTupleStruct = Self;
        type SerializeTupleVariant = Self;
        type SerializeMap = Self;
        type SerializeStruct = Self;
        type SerializeStructVariant = Self;

        fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            match self.ty {
                Some(TypeIdent::SimpleString) => {
                    self.writer.write_all(b"+")?;
                    self.writer.write_all(v.as_bytes())?;
                }
                Some(TypeIdent::BulkString) => {
                    self.writer.write_all(b"$")?;
                    let len = v.chars().count();
                    self.writer.write_all(len.to_string().as_bytes())?;
                    self.writer.write_all(CRLF)?;
                    self.writer.write_all(v.as_bytes())?;
                }
                Some(TypeIdent::Error) => {}
                None => {
                    panic!("unknown type `{:?}`", self.ty);
                }
            }

            Ok(())
        }

        fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            // self.writer.write_all(b"$-1")?;
            // self.writer.write_all(CRLF)?;
            Ok(())
        }

        fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            todo!()
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            match variant {
                "SimpleString" => {
                    self.ty = Some(TypeIdent::SimpleString);
                    value.serialize(&mut *self)?;
                    self.writer.write_all(CRLF)?;
                }
                "BulkString" => {
                    self.ty = Some(TypeIdent::BulkString);
                    value.serialize(&mut *self)?;
                    self.writer.write_all(CRLF)?;
                }
                "Error" => {
                    self.ty = Some(TypeIdent::Error);
                    value.serialize(&mut *self)?;
                    self.writer.write_all(CRLF)?;
                }
                _ => panic!("unknown variant `{}`", variant),
            }
            Ok(())
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            self.writer.write_all(b"*")?;
            let len = len.ok_or_else(|| Error::ExpectedKnownLength)?;
            self.writer.write_all(len.to_string().as_bytes())?;
            self.writer.write_all(CRLF)?;
            Ok(self)
        }

        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            todo!()
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            todo!()
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            todo!()
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            todo!()
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            self.writer.write_all(variant.to_uppercase().as_bytes())?;
            Ok(self)
        }
    }

    impl<'a, W> serde::ser::SerializeSeq for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();

        type Error = Error;

        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            value.serialize(&mut **self)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeTuple for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();

        type Error = Error;

        fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeTupleStruct for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeTupleVariant for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeMap for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;

        fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeStruct for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            _key: &'static str,
            _value: &T,
        ) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            todo!()
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }

    impl<'a, W> serde::ser::SerializeStructVariant for &'a mut Serializer<W>
    where
        W: Write,
    {
        type Ok = ();
        type Error = Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            _key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            value.serialize(&mut **self)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.writer.write_all(CRLF)?;
            Ok(())
        }
    }
}

mod deserialize {
    use super::*;
    use serde::de::{Deserialize, DeserializeSeed, SeqAccess, Visitor};
    use std::{
        error, fmt,
        io::{self, Read},
        num, string,
    };

    #[derive(Eq, PartialEq, Debug)]
    pub enum Error {
        ExpectedByte,
        ExpectedDigits { found: String },
        ExpectedUtf8Encoded,

        ExpectedIntegerTypeIdent,
        ExpectedStringTypeIdent,
        ExpectedArrayIdent,

        Other(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::ExpectedDigits { found } => write!(f, "expected digits, found `{}`", found),
                _ => todo!(),
            }
        }
    }

    impl error::Error for Error {}

    impl serde::de::Error for Error {
        fn custom<T>(msg: T) -> Self
        where
            T: fmt::Display,
        {
            Error::Other(msg.to_string())
        }
    }

    impl From<io::Error> for Error {
        fn from(_: io::Error) -> Self {
            todo!()
        }
    }

    impl From<string::FromUtf8Error> for Error {
        fn from(_: string::FromUtf8Error) -> Self {
            Error::ExpectedUtf8Encoded
        }
    }

    impl From<num::ParseIntError> for Error {
        fn from(_: num::ParseIntError) -> Self {
            todo!()
        }
    }

    pub struct Deserializer<'de, R>
    where
        R: Read,
    {
        reader: &'de mut R,
    }

    impl<'de, R> Deserializer<'de, R>
    where
        R: Read,
    {
        pub fn from_reader(reader: &'de mut R) -> Self {
            Deserializer { reader }
        }
    }

    trait ReadExt: Read {
        type Error: error::Error + From<string::FromUtf8Error> + From<num::ParseIntError>;

        /// Read until CRLF or EOF.
        fn read_until_crlf(&mut self) -> Result<Vec<u8>, Self::Error>;

        fn read_exact_size(&mut self, size: usize) -> Result<Vec<u8>, Self::Error>;

        fn read_to_string_until_crlf(&mut self) -> Result<String, Self::Error> {
            let mut buf = self.read_until_crlf()?;
            buf.trim_crlf();
            let out = String::from_utf8(buf)?;
            Ok(out)
        }

        fn read_exact_size_to_string(&mut self, size: usize) -> Result<String, Self::Error> {
            let mut buf = self.read_exact_size(size)?;
            buf.trim_crlf();
            let out = String::from_utf8(buf)?;
            Ok(out)
        }

        fn read_to_i32_until_crlf(&mut self) -> Result<i32, Self::Error> {
            let s = self.read_to_string_until_crlf()?;
            let len = i32::from_str_radix(&s, 10)?;
            Ok(len)
        }
    }

    impl<R> ReadExt for &mut R
    where
        R: Read,
    {
        type Error = Error;

        fn read_until_crlf(&mut self) -> Result<Vec<u8>, Self::Error> {
            let bytes = self.bytes();
            let mut buf = Vec::new();
            for byte in bytes {
                let byte = byte?;
                buf.push(byte);
                if buf.len() >= 2 && &buf[(buf.len() - 2)..] == CRLF {
                    break;
                }
            }
            Ok(buf)
        }

        fn read_exact_size(&mut self, size: usize) -> Result<Vec<u8>, Self::Error> {
            let mut buf = vec![0; size];
            self.read_exact(&mut buf)?;
            Ok(buf)
        }
    }

    trait BytesExt {
        /// Removes trailing CRLF.
        fn trim_crlf(&mut self);
    }

    impl BytesExt for Vec<u8> {
        fn trim_crlf(&mut self) {
            let len = self.len();
            if len >= 2 {
                self.truncate(len - 2);
            }
        }
    }

    struct TypeLength(usize);

    impl TypeLength {
        fn read(reader: &mut impl Read) -> Result<Self, Error> {
            let bytes = reader.bytes();
            let mut buf = Vec::new();
            for byte in bytes {
                let byte = byte?;
                buf.push(byte);
                if buf.len() >= 2 && &buf[(buf.len() - 2)..] == CRLF {
                    break;
                }
            }
            // Trim CRLF.
            buf.truncate(buf.len() - 2);
            let s = String::from_utf8(buf)?;
            let len =
                usize::from_str_radix(&s, 10).map_err(|_| Error::ExpectedDigits { found: s })?;
            Ok(TypeLength(len))
        }
    }

    enum TypeIdent {
        // +hello\r\n
        SimpleString,
        // -uhoh\r\n
        Error,
        // :123\r\n
        Integer,
        // $5\r\nhello\r\n
        BulkString(TypeLength),
        // *1\r\n+hello\r\n
        Array(TypeLength),
    }

    impl TypeIdent {
        fn read(reader: &mut impl Read) -> Result<Self, Error> {
            let mut bytes = reader.bytes();
            let first_byte = bytes.next().ok_or_else(|| Error::ExpectedByte)??;
            let ty = match &first_byte {
                b'+' => TypeIdent::SimpleString,
                b'-' => TypeIdent::Error,
                b':' => TypeIdent::Integer,
                b'$' => {
                    let len = TypeLength::read(reader)?;
                    TypeIdent::BulkString(len)
                }
                b'*' => {
                    let len = TypeLength::read(reader)?;
                    TypeIdent::Array(len)
                }
                _ => {
                    // return Err(Error::UnknownType)
                    panic!(format!(
                        "unknown type, first byte is `{}`",
                        first_byte as char
                    ))
                }
            };
            Ok(ty)
        }
    }

    pub fn from_reader<'a, T>(reader: &'a mut impl Read) -> Result<T, Error>
    where
        T: Deserialize<'a>,
    {
        let mut deserializer = Deserializer::from_reader(reader);
        let t = T::deserialize(&mut deserializer)?;
        Ok(t)
    }

    impl<'de, 'a, R> serde::de::Deserializer<'de> for &'a mut Deserializer<'de, R>
    where
        R: Read,
    {
        type Error = Error;

        fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let ty = TypeIdent::read(self.reader)?;
            match ty {
                TypeIdent::Integer => {
                    let mut buf = self.reader.read_until_crlf()?;
                    buf.trim_crlf();
                    let buf = String::from_utf8(buf)?;
                    let num = i32::from_str_radix(&buf, 10)
                        .map_err(|_| Error::ExpectedDigits { found: buf })?;
                    visitor.visit_i32(num)
                }
                _ => Err(Error::ExpectedIntegerTypeIdent),
            }
        }

        fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let ty = TypeIdent::read(self.reader)?;
            match ty {
                TypeIdent::SimpleString | TypeIdent::Error => {
                    let v = self.reader.read_to_string_until_crlf()?;
                    visitor.visit_string(v)
                }
                TypeIdent::BulkString(TypeLength(len)) => {
                    // Read len + CRLF length bytes.
                    let v = self.reader.read_exact_size_to_string(len + 2)?;
                    visitor.visit_string(v)
                }
                _ => Err(Error::ExpectedStringTypeIdent),
            }
        }

        fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_unit_struct<V>(
            self,
            _name: &'static str,
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_newtype_struct<V>(
            self,
            _name: &'static str,
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let ty = TypeIdent::read(self.reader)?;
            match ty {
                TypeIdent::Array(TypeLength(len)) => visitor.visit_seq(Array::new(&mut self, len)),
                _ => Err(Error::ExpectedArrayIdent),
            }
        }

        fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_tuple_struct<V>(
            self,
            _name: &'static str,
            _len: usize,
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_struct<V>(
            self,
            _name: &'static str,
            _fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_enum<V>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }

        fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            todo!()
        }
    }

    struct Array<'a, 'de: 'a, R>
    where
        R: Read,
    {
        de: &'a mut Deserializer<'de, R>,
        length: usize,
        element_count: usize,
    }

    impl<'a, 'de, R> Array<'a, 'de, R>
    where
        R: Read,
    {
        fn new(de: &'a mut Deserializer<'de, R>, length: usize) -> Self {
            Array {
                de,
                length,
                element_count: 0,
            }
        }
    }

    impl<'de, 'a, R> SeqAccess<'de> for Array<'a, 'de, R>
    where
        R: Read,
    {
        type Error = Error;

        fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where
            T: DeserializeSeed<'de>,
        {
            if self.element_count == self.length {
                return Ok(None);
            }
            self.element_count += 1;
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_deserialize_simple_string() {
            let input = b"+hello world\r\n";
            let mut input = std::io::Cursor::new(input);

            let output: String = deserialize::from_reader(&mut input).unwrap();

            assert_eq!("hello world", output);
        }

        #[test]
        fn test_deserialize_error() {
            let input = b"-hello world\r\n";
            let mut input = std::io::Cursor::new(input);

            let output: String = deserialize::from_reader(&mut input).unwrap();

            assert_eq!("hello world", output);
        }

        #[test]
        fn test_deserialize_integer() {
            let input = b":123\r\n";
            let mut input = std::io::Cursor::new(input);

            let output: i32 = deserialize::from_reader(&mut input).unwrap();

            assert_eq!(123, output);
        }

        #[test]
        fn test_deserialize_bulk_string() {
            let input = b"$11\r\nhello world\r\n";
            let mut input = std::io::Cursor::new(input);

            let output: String = deserialize::from_reader(&mut input).unwrap();

            assert_eq!("hello world", output);
        }

        #[test]
        fn test_deserialize_array_homogenous() {
            let input = b"*2\r\n$11\r\nhello world\r\n$12\r\nhow are you?\r\n";
            let mut input = std::io::Cursor::new(input);

            let output: Vec<String> = deserialize::from_reader(&mut input).unwrap();

            assert_eq!(
                vec!["hello world".to_owned(), "how are you?".to_owned()],
                output
            );
        }
    }
}
