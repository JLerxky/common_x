use std::{
    io::{Read, Write},
    path::Path,
};

use async_std::{
    fs::{self, OpenOptions},
    io::{BufWriter, WriteExt},
};
use color_eyre::Result;
use tracing::error;

pub fn write_file(content: &[u8], path: impl AsRef<std::path::Path>) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())
        .unwrap_or_else(|_| panic!("open file({:?}) failed.", path.as_ref().to_str()));
    file.write_all(content).unwrap();
}

pub fn touch_file(path: impl AsRef<std::path::Path>) {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.as_ref())
        .unwrap_or_else(|_| panic!("touch file({:?}) failed.", path.as_ref().to_str()));
}

pub fn read_file(path: impl AsRef<std::path::Path>) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

pub async fn write_new_file(path: String, buf: Vec<u8>) -> Result<()> {
    let path = Path::new(&path);
    fs::create_dir_all(&path.parent().unwrap())
        .await
        .map_err(|e| {
            error!("create dir({:?}) failed: {e}", path.to_str());
            e
        })?;
    let f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(|e| {
            error!("open file({:?}) failed: {e}", path.to_str());
            e
        })?;
    let mut buffer = BufWriter::new(f);

    buffer.write_all(&buf).await?;

    buffer.flush().await?;
    Ok(())
}
