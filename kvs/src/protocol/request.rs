use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Deserialize, Serialize, Clone, Debug)]
pub enum Request {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::format::Serialization;
    use quickcheck_macros::quickcheck;
    use std::io::Cursor;

    impl quickcheck::Arbitrary for Request {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            match g.size() % 3 {
                0 => Request::Set {
                    key: String::arbitrary(g),
                    value: String::arbitrary(g),
                },
                1 => Request::Get {
                    key: String::arbitrary(g),
                },
                3 => Request::Rm {
                    key: String::arbitrary(g),
                },
                _ => unimplemented!(),
            }
        }
    }

    #[quickcheck]
    fn prop_ser_de_is_identical(request: Request) -> bool {
        let mut buf = Cursor::new(Vec::new());
        request.to_writer(&mut buf).unwrap();

        buf.set_position(0);
        request == Request::from_reader(&mut buf).unwrap().unwrap()
    }
}
