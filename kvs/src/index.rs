use super::{serialization::Serializable, Result};
use crate::command::Command;
use std::{
    collections::HashMap,
    io::{BufRead, Seek, SeekFrom},
};

pub fn build_index<T>(reader: &mut T) -> Result<HashMap<String, u64>>
where
    T: BufRead + Seek,
{
    reader.seek(SeekFrom::Start(0))?;
    let mut index = HashMap::new();
    loop {
        let offset = reader.seek(SeekFrom::Current(0))?;

        // Check EOF
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let command: Command = Command::deserialize_from(&mut *reader)?;
        match command {
            Command::Set(set) => {
                index.insert(set.key, offset);
            }
            Command::Rm(rm) => {
                index.remove(&rm.key);
            }
        };
    }
    Ok(index)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::command::*;
    use std::io::Cursor;

    #[test]
    fn test_build_index() {
        let commands: Vec<Command> = vec![
            Command::Set(Set {
                key: "key0".to_owned(),
                value: "value0".to_owned(),
            }),
            Command::Set(Set {
                key: "key1".to_owned(),
                value: "value1".to_owned(),
            }),
            Command::Set(Set {
                key: "key2".to_owned(),
                value: "value2".to_owned(),
            }),
            Command::Rm(Rm {
                key: "key2".to_owned(),
            }),
            Command::Set(Set {
                key: "key3".to_owned(),
                value: "value3".to_owned(),
            }),
            Command::Set(Set {
                key: "key3".to_owned(),
                value: "value33".to_owned(),
            }),
        ];
        let mut serialized = Vec::new();
        for cmd in commands.iter() {
            cmd.serialize_into(&mut serialized).unwrap();
        }

        let mut reader = Cursor::new(&serialized);
        let index = build_index(&mut reader).unwrap();

        assert_eq!(index.get("key0"), Some(&0));

        let size = {
            let mut buf = Vec::new();
            commands.get(0).unwrap().serialize_into(&mut buf).unwrap();
            buf.len() as u64
        };
        assert_eq!(index.get("key1"), Some(&size));

        assert_eq!(index.get("key2"), None);

        let size = {
            let mut buf = Vec::new();
            for cmd in &commands[0..5] {
                cmd.serialize_into(&mut buf).unwrap();
            }
            buf.len() as u64
        };
        assert_eq!(index.get("key3"), Some(&size));
    }
}
