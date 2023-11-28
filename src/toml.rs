use std::{fs, io::Write, path::Path};

use color_eyre::{eyre::eyre, Result};
use serde::Deserialize;

pub fn read_toml<'a, T: Deserialize<'a>>(path: impl AsRef<Path>) -> Result<T> {
    let s = fs::read_to_string(&path)
        .map_err(|e| eyre!("read_toml({:?}) err: {e}", path.as_ref().to_str()))?;
    T::deserialize(toml::Deserializer::new(&s)).map_err(|e| eyre!("config deserialize err: {e}"))
}

pub fn write_toml<T: serde::Serialize>(content: T, path: impl AsRef<Path>) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())
        .unwrap_or_else(|_| panic!("open file({:?}) failed.", path.as_ref().to_str()));
    file.write_all(toml::to_string_pretty(&content)?.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}
