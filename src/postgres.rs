use anyhow::Result;
use postgresql_commands::{pg_dump, psql::PsqlBuilder, AsyncCommandExecutor, CommandBuilder};
use regex::Regex;
use std::path::Path;

use crate::config::DatabaseConfig;

pub struct PostgresManager<'a> {
    config: &'a DatabaseConfig,
}

impl<'a> PostgresManager<'a> {
    pub fn new(config: &'a DatabaseConfig) -> Self {
        Self { config }
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        let output = self.execute_psql_command("\\l", None).await?;
        let re = Regex::new(r"\n ([^ ]+)").unwrap();
        let mut databases = Vec::new();

        for cap in re.captures_iter(&output) {
            if let Some(name) = cap.get(1) {
                let name = name.as_str();
                if !["template0", "template1", "postgres"].contains(&name) {
                    databases.push(name.to_string());
                }
            }
        }

        Ok(databases)
    }

    pub async fn create_database(&self, db_name: &str) -> Result<()> {
        self.execute_psql_command(&format!("CREATE DATABASE \"{}\";", db_name), None)
            .await?;
        Ok(())
    }

    pub async fn dump_database(&self, db_name: &str, output_path: &Path) -> Result<()> {
        if let Some(dir) = output_path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }

        let mut dump_builder = pg_dump::PgDumpBuilder::new()
            .dbname(db_name)
            .host(&self.config.host)
            .port(self.config.port)
            .username(&self.config.username)
            .file(output_path);

        if let Some(pwd) = &self.config.password {
            dump_builder = dump_builder.pg_password(pwd);
        }

        let mut dump = dump_builder.build_tokio();
        dump.spawn()?.wait().await?;
        Ok(())
    }

    pub async fn restore_database(&self, db_name: &str, dump_path: &Path) -> Result<()> {
        let mut psql_builder = PsqlBuilder::new()
            .file(dump_path)
            .host(&self.config.host)
            .port(self.config.port)
            .dbname(db_name)
            .username(&self.config.username);

        if let Some(pwd) = &self.config.password {
            psql_builder = psql_builder.pg_password(pwd);
        }

        let mut psql = psql_builder.build_tokio();
        psql.execute(None).await?;
        Ok(())
    }

    async fn execute_psql_command(&self, command: &str, db_name: Option<&str>) -> Result<String> {
        let mut builder = PsqlBuilder::new()
            .command(command)
            .host(&self.config.host)
            .port(self.config.port)
            .username(&self.config.username);

        if let Some(db) = db_name {
            builder = builder.dbname(db);
        }

        if let Some(pwd) = &self.config.password {
            builder = builder.pg_password(pwd);
        }

        let mut command = builder.build_tokio();
        let (output, _) = command.execute(None).await?;
        Ok(output)
    }
}
