use futures::future::BoxFuture;
use sqlx::{
    error::BoxDynError,
    migrate::{Migration, MigrationSource, MigrationType},
    sqlite::{SqlitePoolOptions, SqliteRow},
    Database, Pool, Sqlite, SqlitePool,
};
use std::{borrow::Cow, marker::PhantomData, sync::Arc, time::Duration};
use tracing::info;

use crate::config::rekcod_server_config;

pub(crate) mod kvs;

pub(crate) async fn repository() -> &'static Repository {
    static DB: tokio::sync::OnceCell<Repository> = tokio::sync::OnceCell::const_new();
    let config = rekcod_server_config();
    DB.get_or_init(|| async {
        Repository::new(&config.db_url)
            .await
            .expect("An db client error occured")
    })
    .await
}

pub struct Repository {
    pool: Arc<SqlitePool>,
    pub kvs: DbSet<'static, Sqlite, SqliteRow, kvs::Kvs>,
}

impl Repository {
    async fn new(url: &str) -> anyhow::Result<Self> {
        let conn = SqlitePoolOptions::new()
            .max_connections(50)
            .min_connections(3) // Minimum number of idle connections
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await?;

        let pool = Arc::new(conn);

        Ok(Self {
            pool: Arc::clone(&pool),
            kvs: DbSet::new(Arc::clone(&pool)),
        })
    }
}

pub struct DbSet<'a, DB, ROW, T>
where
    DB: Database,
    ROW: sqlx::Row,
    T: Sized,
{
    pool: Arc<Pool<DB>>,
    _dbset: PhantomData<&'a T>,
    _marker: PhantomData<&'a ROW>,
}

impl<'a, DB, ROW, T> DbSet<'a, DB, ROW, T>
where
    DB: Database,
    ROW: sqlx::Row,
    T: Sized,
{
    fn new(pool: Arc<Pool<DB>>) -> Self {
        DbSet {
            pool,
            _dbset: PhantomData,
            _marker: PhantomData,
        }
    }
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
pub(crate) async fn migrate() -> anyhow::Result<()> {
    let pool = repository().await.pool.as_ref();
    // Enable WAL mode
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(pool)
        .await?;

    // Enable SQLite's query result caching
    // PRAGMA cache_size = -2000; -- Set cache size to 2MB
    // PRAGMA temp_store = MEMORY; -- Store temporary tables and indices in memory
    sqlx::query("PRAGMA cache_size = -2000;")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA temp_store = MEMORY;")
        .execute(pool)
        .await?;

    // migrate
    sqlx::migrate::Migrator::new(Migrations)
        .await?
        .run(pool)
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
