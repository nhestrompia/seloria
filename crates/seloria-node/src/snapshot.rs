use std::path::PathBuf;

use anyhow::Result;
use tokio::io::AsyncWriteExt;

pub async fn download_snapshot(endpoint: &str, out: PathBuf) -> Result<()> {
    let url = format!("{}/snapshot", endpoint);
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Snapshot download failed: {} {}", status, body);
    }

    if let Some(parent) = out.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let bytes = response.bytes().await?;
    let mut file = tokio::fs::File::create(&out).await?;
    file.write_all(&bytes).await?;

    println!("Snapshot saved to {}", out.display());
    Ok(())
}
