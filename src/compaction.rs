use super::{command::Command, serialization::Serializable, Error, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::Seek,
    io::{BufReader, BufWriter, SeekFrom},
};

/// Compact log.
///
/// Compact log by overwriting the log file with known valid entries from log index.
/// `log_file` must have read and write access.
pub fn compact(log_file: &mut File, log_index: &HashMap<String, u64>) -> Result<()> {
    let mut reader = BufReader::new(&*log_file);
    let mut commands = Vec::with_capacity(log_index.len());
    for (_, &offset) in log_index.iter() {
        reader.seek(SeekFrom::Start(offset))?;
        let command = Command::deserialize_from(&mut reader)?;
        match command {
            Command::Set(_) => {
                commands.push(command);
            }
            _ => return Err(Error::IndexDesynced),
        }
    }

    let mut writer = BufWriter::new(&*log_file);
    writer.seek(SeekFrom::Start(0))?;
    for cmd in commands.iter() {
        cmd.serialize_into(&mut writer)?;
    }
    let size = writer.seek(SeekFrom::Current(0))?;

    log_file.set_len(size)?;
    log_file.sync_data()?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{command::*, index::build_index};

    #[test]
    fn test_compact() {
        let commands = vec![
            Command::Set(Set {
                key: "key0".to_owned(),
                value: "value0".to_owned(),
            }),
            Command::Set(Set {
                key: "key1".to_owned(),
                value: "value1".to_owned(),
            }),
            Command::Rm(Rm {
                key: "key0".to_owned(),
            }),
        ];

        let mut log_file = tempfile::tempfile().unwrap();
        for cmd in commands.iter() {
            cmd.serialize_into(&log_file).unwrap();
        }

        let mut reader = BufReader::new(&log_file);
        let log_index = build_index(&mut reader).unwrap();

        compact(&mut log_file, &log_index).unwrap();

        let mut reader = BufReader::new(&log_file);
        reader.seek(SeekFrom::Start(0)).unwrap();
        let log_index = build_index(&mut reader).unwrap();

        let mut expected = HashMap::new();
        expected.insert("key1".to_owned(), 0);
        assert_eq!(log_index, expected);
    }
}
