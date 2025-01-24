use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tokio::fs;

mod config;
mod postgres;
mod webhook;

use crate::config::AppConfig;
use crate::postgres::PostgresManager;
use crate::webhook::WebhookClient;

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Cli {
    /// Restore databases from dump files
    #[clap(long)]
    restore: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let config = AppConfig::from_env()?;
    
    // Create PostgresManager with reference to config's db
    let postgres = PostgresManager::new(&config.db);
    
    if config.restore || args.restore {
        restore(&postgres, &config).await
    } else {
      let webhook = config.webhook_url.as_ref().map(|url| WebhookClient::new(url));
      backup_loop(&postgres, webhook.as_ref(), &config).await
    }
}

async fn backup_loop(
    postgres: &PostgresManager<'_>,
    webhook: Option<&WebhookClient>,
    config: &AppConfig,
) -> Result<()> {
    loop {
        let databases = postgres.list_databases().await?;
        if let Some(webhook) = webhook {
            webhook.send_message(&format!("Dumping {} databases!", databases.len())).await?;
        }

        for db in &databases {
            let dump_path = config.save_path.join(format!("{}.sql", db));
            postgres.dump_database(db, &dump_path).await?;
            
            if let Some(webhook) = webhook {
                webhook.send_file(&dump_path).await?;
            }
        }

        println!("{} databases dumped!", databases.len());

        match config.interval {
            Some(secs) => tokio::time::sleep(Duration::from_secs(secs)).await,
            None => break,
        }
    }
    Ok(())
}

async fn restore(postgres: &PostgresManager<'_>, config: &AppConfig) -> Result<()> {
    let mut entries = fs::read_dir(&config.restore_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            let db_name = path.file_stem().unwrap().to_str().unwrap();
            
            postgres.create_database(db_name).await?;
            postgres.restore_database(db_name, &path).await?;
        }
    }
    Ok(())
}