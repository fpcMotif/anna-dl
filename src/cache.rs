use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json;
use crate::scraper::Book;

#[derive(Clone)]
pub struct SearchCache {
    db_path: PathBuf,
}

impl SearchCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anna-dl");

        Self::at_path(cache_dir)
    }

    pub fn at_path(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)
            .context("Failed to create cache directory")?;

        let db_path = cache_dir.join("cache.db");

        let cache = Self { db_path };
        cache.init()?;

        Ok(cache)
    }

    fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .context("Failed to open cache database")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_results (
                query TEXT PRIMARY KEY,
                results TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        ).context("Failed to create cache table")?;

        Ok(())
    }

    pub fn get(&self, query: &str) -> Result<Option<Vec<Book>>> {
        let conn = Connection::open(&self.db_path)
            .context("Failed to open cache database")?;

        let mut stmt = conn.prepare(
            "SELECT results, timestamp FROM search_results WHERE query = ?"
        )?;

        let mut rows = stmt.query(params![query])?;

        if let Some(row) = rows.next()? {
            let timestamp: u64 = row.get(1)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Expire after 24 hours
            if now - timestamp > 24 * 60 * 60 {
                return Ok(None);
            }

            let results_json: String = row.get(0)?;
            let books: Vec<Book> = serde_json::from_str(&results_json)
                .context("Failed to deserialize cached results")?;

            return Ok(Some(books));
        }

        Ok(None)
    }

    pub fn set(&self, query: &str, books: &[Book]) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .context("Failed to open cache database")?;

        let results_json = serde_json::to_string(books)
            .context("Failed to serialize results")?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        conn.execute(
            "INSERT OR REPLACE INTO search_results (query, results, timestamp)
             VALUES (?, ?, ?)",
            params![query, results_json, timestamp],
        ).context("Failed to cache results")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_cache() -> (SearchCache, PathBuf) {
        let temp_dir = std::env::temp_dir().join(format!(
            "annadl_cache_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test_cache.db");

        let cache = SearchCache { db_path: db_path.clone() };
        cache.init().unwrap();

        (cache, temp_dir)
    }

    #[test]
    fn test_cache_set_and_get() {
        let (cache, temp_dir) = create_test_cache();
        let books = vec![
            Book {
                title: "Test Book".to_string(),
                author: Some("Author".to_string()),
                year: Some("2023".to_string()),
                language: Some("en".to_string()),
                format: Some("pdf".to_string()),
                size: Some("1MB".to_string()),
                url: "http://example.com".to_string(),
            }
        ];

        cache.set("test query", &books).unwrap();

        let cached_books = cache.get("test query").unwrap().unwrap();
        assert_eq!(cached_books.len(), 1);
        assert_eq!(cached_books[0].title, "Test Book");

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_cache_miss() {
        let (cache, temp_dir) = create_test_cache();

        let result = cache.get("nonexistent").unwrap();
        assert!(result.is_none());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_cache_expiry() {
        let (cache, temp_dir) = create_test_cache();

        // Manually insert an expired record
        let conn = Connection::open(&cache.db_path).unwrap();
        let expired_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - (25 * 60 * 60); // 25 hours ago

        conn.execute(
            "INSERT INTO search_results (query, results, timestamp) VALUES (?, ?, ?)",
            params!["expired", "[]", expired_time],
        ).unwrap();

        let result = cache.get("expired").unwrap();
        assert!(result.is_none());

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
