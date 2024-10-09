use axum::{routing::get, Router};
use rekcod_core::constants::REKCOD_SERVER_PREFIX_PATH;

mod server;

pub fn routers() -> Router {
    Router::new()
        .route("/", get(|| async { "server" }))
        .nest(REKCOD_SERVER_PREFIX_PATH, server::routers())
}

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
async fn migrate() -> anyhow::Result<()> {
    Ok(())
}
