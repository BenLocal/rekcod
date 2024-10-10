use sqlx::{prelude::FromRow, sqlite::SqliteRow, Sqlite};

use super::DbSet;

pub struct Kvs;

#[derive(Debug, FromRow)]
pub struct KvsForDb {
    pub id: i64,
    pub module: String,
    pub key: String,
    pub sub_key: String,
    pub third_key: String,
    pub value: String,
}

impl DbSet<'static, Sqlite, SqliteRow, Kvs> {
    pub async fn insert(&self, kvs: &KvsForDb) -> anyhow::Result<()> {
        let _ = sqlx::query!(
            "INSERT INTO kvs (module, key, sub_key, third_key, value) VALUES (?, ?, ?, ?, ?)",
            kvs.module,
            kvs.key,
            kvs.sub_key,
            kvs.third_key,
            kvs.value
        )
        .execute(self.pool.as_ref())
        .await?
        .rows_affected();

        Ok(())
    }

    pub async fn update_value(&self, kvs: &KvsForDb) -> anyhow::Result<()> {
        let _ = sqlx::query!("UPDATE kvs SET value = ? WHERE id = ?", kvs.value, kvs.id)
            .execute(self.pool.as_ref())
            .await?
            .rows_affected();

        Ok(())
    }

    async fn delete(&self, id: i64) -> anyhow::Result<()> {
        let _ = sqlx::query!("DELETE FROM kvs WHERE id = ?", id)
            .execute(self.pool.as_ref())
            .await?
            .rows_affected();

        Ok(())
    }

    async fn get(&self, id: i64) -> anyhow::Result<Option<KvsForDb>> {
        let res = sqlx::query_as!(KvsForDb, "SELECT * FROM kvs WHERE id = ?", id)
            .fetch_optional(self.pool.as_ref())
            .await?;

        Ok(res)
    }

    async fn select(
        &self,
        module: &str,
        key: Option<&str>,
        sub_key: Option<&str>,
        third_key: Option<&str>,
    ) -> anyhow::Result<Vec<KvsForDb>> {
        let mut vs = Vec::new();
        let mut q = format!("SELECT * FROM kvs WHERE module = ?");
        vs.push(module);

        if let Some(key) = key {
            q = format!("{} AND key = ? ", q);
            vs.push(key);
        }

        if let Some(sub_key) = sub_key {
            q = format!("{} AND sub_key = ?", q);
            vs.push(sub_key);
        }

        if let Some(third_key) = third_key {
            q = format!("{} AND third_key = ?", q);
            vs.push(third_key);
        }

        let mut query = sqlx::query_as::<_, KvsForDb>(&q);

        for v in vs {
            query = query.bind(v);
        }

        let res = query.fetch_all(self.pool.as_ref()).await?;
        Ok(res)
    }
}
