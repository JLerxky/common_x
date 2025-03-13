use color_eyre::{Result, eyre::eyre};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
};
use tracing::error;

pub async fn write_file(path: impl AsRef<std::path::Path>, content: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())
        .await
        .map_err(|e| eyre!("open file({:?}) failed: {e}", path.as_ref().to_str()))?;
    file.write_all(content).await?;
    Ok(file.flush().await?)
}

pub async fn touch_file(path: impl AsRef<std::path::Path>) -> Result<()> {
    fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path.as_ref())
        .await
        .map_err(|_| eyre!("touch file({:?}) failed.", path.as_ref().to_str()))?;
    Ok(())
}

pub async fn read_file_to_string(path: impl AsRef<std::path::Path>) -> Result<String> {
    let mut f = fs::File::open(path.as_ref()).await?;
    let mut s = String::new();
    f.read_to_string(&mut s).await?;
    Ok(s)
}

pub async fn create_file(path: impl AsRef<std::path::Path>, buf: &[u8]) -> Result<()> {
    let path = path.as_ref();
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

    buffer.write_all(buf).await?;

    buffer.flush().await?;
    Ok(())
}
