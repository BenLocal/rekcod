use std::{borrow::Cow, path::Path};

use axum::{routing::get, Router};
use futures::future::BoxFuture;
use rekcod_core::{
    auth::set_token,
    constants::{REKCOD_CONFIG_FILE_NAME, REKCOD_SERVER_PREFIX_PATH},
    obj::RekcodCfg,
};

use sqlx::{
    error::BoxDynError,
    migrate::{Migration, MigrationSource, MigrationType},
    SqlitePool,
};
use tokio_util::sync::CancellationToken;
use tracing::info;
use uuid::Uuid;

pub mod config;
mod db;
mod node;
mod server;

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "server" }))
        .nest(REKCOD_SERVER_PREFIX_PATH, server::routers())
}

pub async fn init(_cancel: CancellationToken) -> anyhow::Result<()> {
    init_rekcod_client_config().await?;
    migrate().await?;
    Ok(())
}

#[derive(Debug, rust_embed::Embed)]
#[folder = "migrations/"]
struct Migrations;

/// sqlite
///
/// ``` sql
/// CREATE TABLE IF NOT EXISTS "kvs" (
///	   "id"	INTEGER NOT NULL,
///	   "module"	VARCHAR NOT NULL,
///	   "key"	VARCHAR NOT NULL,
///    "sub_key"	VARCHAR NOT NULL,
///    "thrid_key"	VARCHAR NOT NULL,
///    "value"	TEXT NOT NULL,
///    PRIMARY KEY("id" AUTOINCREMENT)
/// );
///
/// CREATE INDEX IF NOT EXISTS "kvs_module_idx" ON "kvs" ("module");
/// CREATE INDEX IF NOT EXISTS "kvs_module_key_idx" ON "kvs" ("key", "module");
/// CREATE INDEX IF NOT EXISTS "kvs_module_sub_key_idx" ON "kvs" ("sub_key", "key", "module");
/// CREATE INDEX IF NOT EXISTS "kvs_module_third_key_idx" ON "kvs" ("thrid_key", "sub_key", "key", "module");
/// ```
///
async fn migrate() -> anyhow::Result<()> {
    let config = config::rekcod_server_config();
    let db = SqlitePool::connect(config.db_url.as_str()).await?;
    // migrate
    sqlx::migrate::Migrator::new(Migrations)
        .await?
        .run(&db)
        .await?;

    info!("sql init success");
    Ok(())
}

impl MigrationSource<'static> for Migrations {
    fn resolve(self) -> BoxFuture<'static, Result<Vec<Migration>, BoxDynError>> {
        Box::pin(async move {
            let mut migrations = Vec::new();

            for path in Self::iter() {
                let emb_file = Self::get(&path);

                if let Some(emb_file) = emb_file {
                    let parts = std::path::Path::new(path.as_ref())
                        .file_name()
                        .map_or("", |x| x.to_str().unwrap_or(""))
                        .splitn(2, '_')
                        .collect::<Vec<_>>();

                    if parts.len() != 2 || !parts[1].ends_with(".sql") {
                        // not of the format: <VERSION>_<DESCRIPTION>.sql; ignore
                        continue;
                    }
                    let version: i64 = parts[0].parse()?;

                    let migration_type = MigrationType::from_filename(parts[1]);
                    // remove the `.sql` and replace `_` with ` `
                    let description = parts[1]
                        .trim_end_matches(migration_type.suffix())
                        .replace('_', " ")
                        .to_owned();

                    let sql = unsafe { std::str::from_utf8_unchecked(emb_file.data.as_ref()) };

                    migrations.push(Migration::new(
                        version,
                        Cow::Owned(description),
                        migration_type,
                        Cow::Owned(sql.to_owned()),
                        false,
                    ));
                }
            }

            Ok(migrations)
        })
    }
}

async fn init_rekcod_client_config() -> anyhow::Result<()> {
    let config = config::rekcod_server_config();
    let config_dir = Path::new(&config.config_path);
    if !config_dir.exists() {
        tokio::fs::create_dir_all(config_dir).await?;
    }

    // check config file exists
    let cfg_path = config_dir.join(REKCOD_CONFIG_FILE_NAME);
    if cfg_path.exists() {
        // read token from file
        let cfg_str = tokio::fs::read_to_string(&cfg_path).await?;
        let c = serde_json::from_str::<RekcodCfg>(&cfg_str)?;
        set_token(c.token.clone());

        info!("init token success: {}", c.token);
        return Ok(());
    }

    let token = Uuid::new_v4().to_string();
    set_token(token.clone());
    let c = RekcodCfg {
        host: format!("127.0.0.1:{}", config.api_port),
        token: token.clone(),
    };

    tokio::fs::write(cfg_path, serde_json::to_string(&c)?).await?;
    info!("init token success: {}", token);
    Ok(())
}
