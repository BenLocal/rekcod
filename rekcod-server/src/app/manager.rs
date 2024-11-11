use std::{collections::HashMap, path::PathBuf, sync::Arc};

use bollard::container::RemoveContainerOptions;
use once_cell::sync::Lazy;
use rekcod_core::{application::Application, docker::DockerComposeCli};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::{config::rekcod_server_config, db, node::node_manager};

use super::watch::AppWatcher;

static APP_MANAGER: Lazy<AppManager> = Lazy::new(AppManager::new);

pub fn get_app_manager() -> &'static AppManager {
    &APP_MANAGER
}

pub struct AppManager {
    pub app_list: RwLock<HashMap<String, Arc<AppState>>>,
}

pub struct AppState {
    pub tmpl_service: ServeDir,
    pub id: String,
    pub info: Option<Application>,
    pub tmpls: Vec<String>,
    pub watcher: AppWatcher,
    pub values: Option<serde_yaml::Value>,
}

impl AppManager {
    pub fn new() -> AppManager {
        AppManager {
            app_list: HashMap::new().into(),
        }
    }

    pub async fn get_app(&self, id: &str) -> Option<Arc<AppState>> {
        self.app_list.read().await.get(id).map(Arc::clone)
    }

    pub async fn get_app_list(&self) -> Vec<Arc<AppState>> {
        self.app_list
            .read()
            .await
            .values()
            .map(Arc::clone)
            .collect::<Vec<_>>()
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let config = rekcod_server_config();
        let (mut dir_entries, _) = walk_dir(&config.get_app_root_path()).await?;

        let mut apps = self.app_list.write().await;

        while let Some(entry) = dir_entries.pop() {
            let id = match entry.file_name() {
                Some(name) => name.to_string_lossy(),
                None => continue,
            };

            let tmpl_path = entry.join("template");
            let (_, files) = walk_dir(&tmpl_path).await?;
            let tmpls = files
                .into_iter()
                .filter_map(|f| f.file_name().map(|d| d.to_string_lossy().to_string()))
                .collect::<Vec<_>>();

            let application_path = entry.join("application.yaml");
            let file = match std::fs::File::open(&application_path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let application: Option<Application> = match serde_yaml::from_reader(file) {
                Ok(f) => Some(f),
                Err(e) => {
                    tracing::error!("Error loading application.yaml: {}", e);
                    None
                }
            };
            let (app_watcher, mut app_notifier) = AppWatcher::new(&application_path, &tmpl_path)?;
            let app = Arc::new(AppState {
                tmpl_service: ServeDir::new(&tmpl_path),
                id: id.to_string(),
                info: application,
                tmpls,
                watcher: app_watcher,
                values: None,
            });

            let mut app_clone = app.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = app_notifier.changed() => {
                            let file = match std::fs::File::open(&application_path) {
                                Ok(f) => f,
                                Err(_) => continue,
                            };
                            let application: Application = match serde_yaml::from_reader(file) {
                                Ok(f) => f,
                                Err(e) => {
                                    println!("Error loading application.yaml: {}", e);
                                    continue;
                                }
                            };
                            {
                              if let Some(tmp) = Arc::get_mut(&mut app_clone){
                                tmp.info = Some(application);
                              }
                            }
                        }
                    }
                }
            });

            apps.insert(id.to_string(), app);
        }

        Ok(())
    }
}

async fn walk_dir(path: &PathBuf) -> anyhow::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut entries = tokio::fs::read_dir(path).await?;

    let mut sub_dirs = Vec::new();
    let mut sub_files = Vec::new();
    loop {
        match entries.next_entry().await {
            Ok(Some(entry)) => {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    sub_dirs.push(entry_path);
                } else {
                    sub_files.push(entry_path);
                }
            }
            Ok(None) => break,
            Err(e) => return Err(anyhow::anyhow!(e.to_string()).into()),
        }
    }

    Ok((sub_dirs, sub_files))
}

#[derive(Serialize, Deserialize, Debug)]
struct AppDeployInfo {
    pub name: String,
    pub node_name: String,
}

pub async fn deploy(name: &str, node_name: &str, app: &AppState) -> anyhow::Result<()> {
    let repositry = db::repository().await;
    let db_app = repositry
        .kvs
        .select_one("app", Some(name), None, None)
        .await?;
    let (insert, info): (bool, AppDeployInfo) = match db_app {
        Some(info) => (false, serde_json::from_str(&info.value)?),
        None => (
            true,
            AppDeployInfo {
                name: name.to_string(),
                node_name: node_name.to_string(),
            },
        ),
    };

    // deploy
    // 1. stop old app
    if !insert {
        // get old app
        let old_node = node_manager().get_node(&info.node_name).await?;
        if let Some(old_node) = old_node {
            let docker_cli = old_node.docker.clone();
            let options = Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            });
            docker_cli.remove_container(name, options).await?;
        }
    }
    // 2. prepare new app, copy files to node
    let ctx = &app.values.clone().unwrap_or_default();
    for tmpl in app.tmpls.iter() {
        let context = app.watcher.get_context(tmpl, ctx)?;
        tokio::fs::create_dir_all("./tmp").await?;
        tokio::fs::write(format!("./tmp/{}", tmpl), &context).await?;
    }

    // 3. start new app
    let new_node = node_manager().get_node(node_name).await?;
    if let Some(new_node) = new_node {
        let mut docker_compose_cli = DockerComposeCli::new(
            new_node.get_node_ip(),
            new_node.get_node_port(),
            &["up", "-d"],
        )?;
        docker_compose_cli.run().await?;
    }

    if insert {
        repositry
            .kvs
            .insert(&db::kvs::KvsForDb {
                module: "app".to_string(),
                key: name.to_string(),
                value: serde_json::to_string(&info)?,
                ..Default::default()
            })
            .await?;
    } else {
        repositry
            .kvs
            .update_value(&db::kvs::KvsForDb {
                module: "app".to_string(),
                key: name.to_string(),
                value: serde_json::to_string(&info)?,
                ..Default::default()
            })
            .await?;
    }

    Ok(())
}
