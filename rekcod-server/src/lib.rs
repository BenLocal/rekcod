use std::borrow::Cow;

use axum::{routing::get, Router};
use db::kvs::KvsForDb;
use futures::future::BoxFuture;
use rekcod_core::constants::REKCOD_SERVER_PREFIX_PATH;

use sqlx::{
    error::BoxDynError,
    migrate::{Migration, MigrationSource, MigrationType},
    SqlitePool,
};
use tokio_util::sync::CancellationToken;
use tracing::info;

pub mod config;
mod db;
mod server;

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "server" }))
        .nest(REKCOD_SERVER_PREFIX_PATH, server::routers())
}

pub async fn init(_cancel: CancellationToken) -> anyhow::Result<()> {
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

    // test
    let db = db::repository().await;
    db.kvs
        .insert(&KvsForDb {
            id: 0,
            module: "server".to_owned(),
            key: "server".to_owned(),
            sub_key: "server".to_owned(),
            third_key: "server".to_owned(),
            value: "server".to_owned(),
        })
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
