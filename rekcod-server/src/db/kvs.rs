use sqlx::{prelude::FromRow, sqlite::SqliteRow, Sqlite};

use super::DbSet;

pub struct Kvs;

#[derive(Debug, FromRow, Default)]
pub struct KvsForDb {
    #[allow(dead_code)]
    pub id: i64,
    pub module: String,
    pub key: String,
    pub sub_key: String,
    pub third_key: String,
    pub value: String,
}

impl DbSet<'static, Sqlite, SqliteRow, Kvs> {
    pub async fn insert(&self, kvs: &KvsForDb) -> anyhow::Result<()> {
        let _ = sqlx::query(
            "INSERT INTO kvs (module, key, sub_key, third_key, value) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(kvs.module.as_str())
        .bind(kvs.key.as_str())
        .bind(kvs.sub_key.as_str())
        .bind(kvs.third_key.as_str())
        .bind(kvs.value.as_str())
        .execute(self.pool.as_ref())
        .await?
        .rows_affected();

        Ok(())
    }

    pub async fn update_value(&self, kvs: &KvsForDb) -> anyhow::Result<()> {
        let _ = sqlx::query("UPDATE kvs SET value = ? WHERE module = ? AND key = ?")
            .bind(kvs.value.as_str())
            .bind(kvs.module.as_str())
            .bind(kvs.key.as_str())
            .execute(self.pool.as_ref())
            .await?
            .rows_affected();

        Ok(())
    }

    pub async fn insert_or_update_value(&self, kvs: &KvsForDb) -> anyhow::Result<()> {
        let _ = sqlx::query(
            r#"INSERT INTO kvs (module, key, sub_key, third_key, value) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (module, key, sub_key, third_key) DO
            UPDATE SET value = EXCLUDED.value
            "#,
        )
        .bind(kvs.module.as_str())
        .bind(kvs.key.as_str())
        .bind(kvs.sub_key.as_str())
        .bind(kvs.third_key.as_str())
        .bind(kvs.value.as_str())
        .execute(self.pool.as_ref())
        .await?
        .rows_affected();

        Ok(())
    }

    pub async fn delete(
        &self,
        module: &str,
        key: Option<&str>,
        sub_key: Option<&str>,
        third_key: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut vs = Vec::new();
        let mut q = format!("DELETE FROM kvs WHERE module = ?");
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

        q = format!("{} limit 1 ", q);

        let mut query = sqlx::query(&q);
        for v in vs {
            query = query.bind(v);
        }

        let _ = query.execute(self.pool.as_ref()).await?;
        Ok(())
    }

    pub async fn select_one(
        &self,
        module: &str,
        key: Option<&str>,
        sub_key: Option<&str>,
        third_key: Option<&str>,
    ) -> anyhow::Result<Option<KvsForDb>> {
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

        q = format!("{} limit 1 ", q);

        let mut query = sqlx::query_as::<_, KvsForDb>(&q);
        for v in vs {
            query = query.bind(v);
        }

        let res = query.fetch_optional(self.pool.as_ref()).await?;
        Ok(res)
    }

    pub async fn select(
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
