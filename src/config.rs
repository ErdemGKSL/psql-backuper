use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub db: DatabaseConfig,
    pub interval: Option<u64>,
    pub restore: bool,
    pub save_path: PathBuf,
    pub restore_path: PathBuf,
    pub webhook_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let db = DatabaseConfig {
            host: std::env::var("PG_HOST").unwrap_or_else(|_| "localhost".into()),
            port: std::env::var("PG_PORT")
                .map(|s| s.parse().unwrap_or(5432))
                .unwrap_or(5432),
            username: std::env::var("PG_USERNAME").unwrap_or_else(|_| "postgres".into()),
            password: std::env::var("PG_PASSWORD").ok(),
        };

        let save_path = std::env::var("SAVE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap().join("dumps"));
        
        let restore_path = std::env::var("RESTORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap().join("dumps"));

        let interval = std::env::var("INTERVAL").ok().and_then(|s| s.parse().ok());

        let restore = std::env::var("RESTORE")
            .map(|s| s == "true")
            .unwrap_or(false)
            || std::env::args().any(|arg| arg == "restore");

        let webhook_url = std::env::var("WEBHOOK_URL")
            .ok()
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });

        Ok(Self {
            db,
            interval,
            restore,
            save_path,
            restore_path,
            webhook_url,
        })
    }
}