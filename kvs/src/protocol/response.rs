use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Deserialize, Serialize, Clone, Debug)]
pub enum Response {
    Success(Option<String>),
    Failure(String),
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Serialization;
    use quickcheck_macros::quickcheck;
    use std::io::Cursor;

    impl quickcheck::Arbitrary for Response {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            match g.size() % 2 {
                0 => Response::Success(if g.size() % 2 == 0 {
                    None
                } else {
                    Some(String::arbitrary(g))
                }),
                1 => Response::Failure(String::arbitrary(g)),
                _ => unimplemented!(),
            }
        }
    }

    #[quickcheck]
    fn prop_ser_de_is_identical(response: Response) -> bool {
        let mut buf = Cursor::new(Vec::new());
        response.to_writer(&mut buf).unwrap();

        buf.set_position(0);
        response == Response::from_reader(&mut buf).unwrap().unwrap()
    }
}
