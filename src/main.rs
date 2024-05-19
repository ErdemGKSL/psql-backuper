use std::fs;
use std::path::Path;
use std::time::Duration;
use postgresql_commands::psql::PsqlBuilder;
use postgresql_commands::{Result, CommandBuilder, CommandExecutor};
use regex::Regex;
use postgresql_commands::pg_dump;
use dotenv;

use serenity::builder::{ExecuteWebhook, CreateAttachment};
use serenity::http;
use serenity::model::webhook::Webhook;
use tokio::fs::File;

#[tokio::main]
async fn main() {
  let interval = std::env::var("INTERVAL")
    .map(|s| {
      s.parse().ok()
    }).ok().flatten();

  loop {
    execute_process().await;
    if let Some(interval) = interval {
      tokio::time::sleep(Duration::from_secs(
        interval
      )).await;
    } else {
      break;
    }
  }
}

async fn execute_process() {
  dotenv::dotenv().ok();
  let (out_list, _) = exec_sql("\\l").unwrap();

  let mut database_names = Vec::new();
  let re = Regex::new(r"\n ([^ ]+)").unwrap();
  
  for cap in re.captures_iter(&out_list) {
    if let Some(name) = cap.get(1) {
      let name = name.as_str();
      if !(vec!["template", "postgres", "template0", "template1", "template2"].into_iter().any(|i|i.eq(name))) {
        database_names.push(name.to_string());
      }
    }
  }

  let save_path_string = std::env::var("SAVE_PATH").unwrap_or(
    std::env::current_dir()
      .unwrap()
      .join("./dumps").to_str().unwrap().to_owned()
  );

  {
    let save_path = Path::new(&save_path_string);
    let _ = fs::create_dir_all(save_path);
  }

  let Credentials { host, port, username, password } = get_credentials();

  for db in database_names.clone() {
    let save_path = Path::new(&save_path_string);
    let save_path = save_path.join(format!("./{db}.sql"));
    let mut dump_builder = pg_dump::PgDumpBuilder::new()
      .dbname(&db)
      .host(&host)
      .port(port.clone())
      .username(&username)
      .file(&save_path);

    if let Some(pwd) = &password {
      dump_builder = dump_builder.pg_password(pwd);
    }

    let mut dump = dump_builder.build();

    let dump = dump.spawn();

    let mut joins = vec![];

    if let Ok(mut dump) = dump {
      joins.push(tokio::spawn(async move {
        let _ = dump.wait();
        tokio::time::sleep(Duration::from_secs(1)).await;
        let file = File::open(save_path).await;
        if let Ok(file) = file {
          let _ = send_file(&file, &format!("./{db}.sql")).await;
        }
      }));
    }

    for join in joins {
      let _ = join.await;
    }
  }
}

async fn send_file(file: &File, file_name: &str) -> Result<(), ()> {
  let webhook_url = std::env::var("WEBHOOK_URL")
    .map_err(|_| (()))?;

  let http = http::Http::new("");

  let webhook = Webhook::from_url(&http, &webhook_url)
    .await
    .map_err(|_| (()))?;

  let x_webhook = ExecuteWebhook::new()
    .add_file(
      CreateAttachment::file(file, file_name).await.map_err(|_| (()))?
    ).username("PSQL BACKUPER");
  let e = webhook.execute(&http, true, x_webhook).await;
  println!("{e:?}");
  Ok(())
}

fn exec_sql(sql: &str) -> Result<(String, String)> {
  let Credentials { host, port, username, password } = get_credentials();

  let mut psql_builder = PsqlBuilder::new()
    .command(sql)
    .host(host)
    .port(port)
    .username(username);

  if let Some(pwd) = password {
    psql_builder = psql_builder.pg_password(pwd);
  }

  let mut psql = psql_builder.build();

  let (stdout, stderr) = psql.execute()?;
  Ok((stdout, stderr))
}

fn get_credentials() -> Credentials {
  let host = std::env::var("PG_HOST")
    .unwrap_or("localhost".to_owned());

  let port = std::env::var("PG_PORT")
    .unwrap_or("5432".to_owned())
    .parse()
    .unwrap_or(5432);

  let username = std::env::var("PG_USERNAME")
    .unwrap_or("postgres".to_owned());

  let password = std::env::var("PG_PASS").ok();

  Credentials {
    host,
    port,
    username,
    password
  }
}

struct Credentials {
  host: String,
  port: u16,
  username: String,
  password: Option<String>
}