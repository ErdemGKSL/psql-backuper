use anyhow::Result;
use serenity::{
    builder::{CreateAttachment, ExecuteWebhook},
    http::Http,
    model::webhook::Webhook,
};
use std::path::Path;
use tokio::fs::File;

pub struct WebhookClient {
    http: Http,
    url: String,
}

impl WebhookClient {
    pub fn new(url: &str) -> Self {
        Self {
            http: Http::new(""),
            url: url.to_string(),
        }
    }

    pub async fn send_message(&self, content: &str) -> Result<()> {
        let webhook = Webhook::from_url(&self.http, &self.url).await?;
        webhook
            .execute(&self.http, true, ExecuteWebhook::new().content(content).username("PSQL BACKUPER"))
            .await?;
        Ok(())
    }

    pub async fn send_file(&self, path: &Path) -> Result<()> {
      let webhook = Webhook::from_url(&self.http, &self.url).await?;
      let file = File::open(path).await?;
      
      // Convert OS string to Rust string with proper error handling
      let file_name = path.file_name()
          .and_then(|n| n.to_str())
          .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
  
      let attachment = CreateAttachment::file(&file, file_name).await?;
  
      webhook
          .execute(&self.http, true, ExecuteWebhook::new().add_file(attachment).username("PSQL BACKUPER"))
          .await?;
      Ok(())
  }
}