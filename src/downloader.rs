use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures::StreamExt;

pub struct Downloader {
    client: reqwest::Client,
    download_path: PathBuf,
}

impl Downloader {
    pub fn new(download_path: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self { client, download_path })
    }
    
    pub async fn download(&self, url: &str, filename: Option<&str>) -> Result<PathBuf> {
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;
        
        let total_size = response
            .content_length()
            .ok_or_else(|| anyhow::anyhow!("Failed to get content length"))?;
        
        let filename = self.determine_filename(url, filename, &response)?;
        let filepath = self.download_path.join(&filename);
        
        tokio::fs::create_dir_all(&self.download_path)
            .await
            .context("Failed to create download directory")?;
        
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}"
                )
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message(format!("Downloading {}", filename));
        
        let mut file = File::create(&filepath)
            .await
            .context("Failed to create file")?;
        
        let mut stream = response.bytes_stream();
        let mut downloaded = 0;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to download chunk")?;
            file.write_all(&chunk).await.context("Failed to write chunk")?;
            
            downloaded = std::cmp::min(downloaded + chunk.len() as u64, total_size);
            pb.set_position(downloaded);
        }
        
        pb.finish_with_message(format!("Downloaded {}", filename));
        Ok(filepath)
    }
    
    fn determine_filename(
        &self,
        url: &str,
        provided_name: Option<&str>,
        response: &reqwest::Response,
    ) -> Result<String> {
        if let Some(name) = provided_name {
            return Ok(name.to_string());
        }
        
        if let Some(filename) = Self::extract_filename_from_url(url) {
            return Ok(filename);
        }
        
        if let Some(disposition) = response.headers().get("content-disposition") {
            if let Ok(disposition_str) = disposition.to_str() {
                if let Some(name) = Self::parse_content_disposition(disposition_str) {
                    return Ok(name);
                }
            }
        }
        
        Ok(format!("downloaded_file_{}.tmp", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ))
    }
    
    fn extract_filename_from_url(url: &str) -> Option<String> {
        url.split('/').last()
            .filter(|s| !s.is_empty() && !s.contains('?'))
            .map(|s| {
                urlencoding::decode(s)
                    .unwrap_or_default()
                    .to_string()
            })
    }
    
    fn parse_content_disposition(disposition: &str) -> Option<String> {
        for part in disposition.split(';') {
            let part = part.trim();
            if part.starts_with("filename=") {
                let filename = part.strip_prefix("filename=")?;
                let filename = filename.trim_matches('"');
                return Some(urlencoding::decode(filename).unwrap_or_default().to_string());
            }
            if part.starts_with("filename*=") {
                let filename = part.strip_prefix("filename*=UTF-8''")?;
                return Some(urlencoding::decode(filename).unwrap_or_default().to_string());
            }
        }
        None
    }
    
    pub fn is_download_in_progress(&self, filename: &str) -> bool {
        let temp_path = self.download_path.join(format!("{}.crdownload", filename));
        let partial_path = self.download_path.join(format!("{}.part", filename));
        
        temp_path.exists() || partial_path.exists()
    }
    
    pub async fn cleanup_partial_downloads(&self) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.download_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".crdownload") || filename.ends_with(".part") {
                    tokio::fs::remove_file(path).await?;
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            Downloader::extract_filename_from_url("https://example.com/file.pdf"),
            Some("file.pdf".to_string())
        );
        assert_eq!(
            Downloader::extract_filename_from_url("https://example.com/file%20name.epub"),
            Some("file name.epub".to_string())
        );
    }
    
    #[test]
    fn test_parse_content_disposition() {
        assert_eq!(
            Downloader::parse_content_disposition("attachment; filename=\"test.pdf\""),
            Some("test.pdf".to_string())
        );
        assert_eq!(
            Downloader::parse_content_disposition("attachment; filename*=UTF-8''test%20file.epub"),
            Some("test file.epub".to_string())
        );
    }

    #[tokio::test]
    async fn test_is_download_in_progress() {
        let temp_dir = std::env::temp_dir().join(format!("annadl_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let downloader = Downloader::new(temp_dir.clone()).unwrap();

        let filename = "test_file.pdf";
        let part_file = temp_dir.join(format!("{}.part", filename));

        // No file exists
        assert!(!downloader.is_download_in_progress(filename));

        // Create .part file
        File::create(&part_file).await.unwrap();
        assert!(downloader.is_download_in_progress(filename));

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }
}
