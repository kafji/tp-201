use super::serialization::Serializable;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Set {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Rm {
    pub key: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Command {
    Set(Set),
    Rm(Rm),
}

impl Serializable<'_> for Command {}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::{prelude::*, BufReader, BufWriter, SeekFrom};
    use tempfile::tempfile;

    #[test]
    fn test_ser_de() {
        let commands: Vec<Command> = (0..2)
            .map(|x| {
                Command::Set(Set {
                    key: format!("key{}", x),
                    value: format!("value{}", x),
                })
            })
            .collect();

        let mut file = tempfile().unwrap();

        // Serialize commands into the tempfile
        {
            let mut writer = BufWriter::new(&file);
            for cmd in commands {
                cmd.serialize_into(&mut writer).unwrap();
            }
        }

        // Print out tempfile content
        file.seek(SeekFrom::Start(0)).unwrap();
        {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            println!("file content: {:?}", &buf);
        }

        // Deserialize commands from tempfile
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut commands: Vec<Command> = Vec::new();
        let mut reader = BufReader::new(&file);
        loop {
            let buf = reader.fill_buf().unwrap();
            if buf.is_empty() {
                break;
            };
            let command = Command::deserialize_from(&mut reader).unwrap();
            commands.push(command);
        }
        println!("deserialized: {:?}", &commands);
    }
}
