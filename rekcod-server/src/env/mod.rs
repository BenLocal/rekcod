use std::collections::HashMap;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::db;

static ENV_MANAGER: Lazy<EnvManager> = Lazy::new(EnvManager::new);

pub fn env_manager() -> &'static EnvManager {
    &ENV_MANAGER
}

pub struct EnvManager {
    values: RwLock<Option<HashMap<String, String>>>,
}

impl EnvManager {
    pub fn new() -> Self {
        Self {
            values: RwLock::new(None),
        }
    }

    pub async fn set(&self, txt: &str) -> anyhow::Result<()> {
        let mut values = self.values.write().await;
        let mut map = HashMap::new();
        for line in txt.lines() {
            if line.is_empty() {
                continue;
            }

            if line.starts_with('#') {
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                map.insert(key.to_string(), val.to_string());
            }
        }
        *values = Some(map);
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        {
            let mut values = self.values.write().await;
            if values.is_none() {
                // get from db
                let v = Self::get_from_db().await;
                if let Ok(v) = v {
                    let tmp = v.get(key).cloned();
                    *values = Some(v);
                    return tmp;
                } else {
                    return None;
                }
            }
        }

        self.values.read().await.as_ref()?.get(key).cloned()
    }

    #[allow(dead_code)]
    pub async fn get_all(&self) -> Option<HashMap<String, String>> {
        {
            let mut values = self.values.write().await;
            if values.is_none() {
                // get from db
                let v = Self::get_from_db().await;
                if let Ok(v) = v {
                    let tmp = v.clone();
                    *values = Some(v);
                    return Some(tmp);
                } else {
                    return None;
                }
            }
        }

        self.values.read().await.clone()
    }

    pub async fn get_from_db() -> anyhow::Result<HashMap<String, String>> {
        let db = db::repository().await;
        let env_db = db.kvs.select_one("env", Some("global"), None, None).await?;

        let env = env_db.map(|env| env.value).map_or("".to_string(), |v| v);

        let mut map = HashMap::new();
        for line in env.lines() {
            if line.is_empty() {
                continue;
            }

            if line.starts_with('#') {
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                map.insert(key.to_string(), val.to_string());
            }
        }
        Ok(map)
    }
}
