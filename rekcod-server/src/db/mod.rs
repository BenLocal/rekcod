use std::{marker::PhantomData, sync::Arc};

use sqlx::{sqlite::SqliteRow, Database, Pool, Sqlite, SqlitePool};

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
    pub kvs: DbSet<'static, Sqlite, SqliteRow, kvs::Kvs>,
}

impl Repository {
    async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = Arc::new(SqlitePool::connect(url).await?);

        Ok(Self {
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
