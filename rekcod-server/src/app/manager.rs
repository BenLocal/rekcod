use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{config::rekcod_server_config, db, node::node_manager};
use bollard::container::RemoveContainerOptions;
use once_cell::sync::Lazy;
use rekcod_core::{
    api::req::AppDeployRequest, application::ApplicationTmpl, docker::DockerComposeCli,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::error;

use super::watch::AppWatcher;

static APP_TMPL_MANAGER: Lazy<AppTmplManager> = Lazy::new(AppTmplManager::new);

pub fn get_app_tmpl_manager() -> &'static AppTmplManager {
    &APP_TMPL_MANAGER
}

pub struct AppTmplManager {
    pub app_tmpl_list: RwLock<HashMap<String, Arc<AppTmplState>>>,
}

pub struct AppTmplState {
    root_path: PathBuf,
    pub tmpl_service: ServeDir,
    pub id: String,
    pub info: Option<ApplicationTmpl>,
    pub tmpls: Vec<String>,
    pub watcher: AppWatcher,
}

impl AppTmplState {
    pub fn get_app_root_path(&self) -> &PathBuf {
        &self.root_path
    }

    pub fn get_default_project_path(&self) -> PathBuf {
        self.get_app_root_path().join("project")
    }
}

impl AppTmplManager {
    pub fn new() -> Self {
        Self {
            app_tmpl_list: HashMap::new().into(),
        }
    }

    pub async fn get_app_tmpl(&self, id: &str) -> Option<Arc<AppTmplState>> {
        self.app_tmpl_list.read().await.get(id).map(Arc::clone)
    }

    pub async fn get_app_tmpl_list(&self) -> Vec<Arc<AppTmplState>> {
        self.app_tmpl_list
            .read()
            .await
            .values()
            .map(Arc::clone)
            .collect::<Vec<_>>()
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let config = rekcod_server_config();
        let get_app_root_path = config.get_app_root_path();
        if !get_app_root_path.exists() {
            tokio::fs::create_dir_all(&get_app_root_path).await?;
        }

        let (mut dir_entries, _) = walk_dir(&get_app_root_path).await?;

        let mut app_tmpls = self.app_tmpl_list.write().await;

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
            let content = match tokio::fs::read_to_string(&application_path).await {
                Ok(f) => f,
                Err(_) => continue,
            };
            let application_tmpl: Option<ApplicationTmpl> = match serde_yaml::from_str(&content) {
                Ok(f) => Some(f),
                Err(e) => {
                    tracing::error!("Error loading application.yaml: {}", e);
                    None
                }
            };
            let (app_watcher, mut app_notifier) =
                match AppWatcher::new(&application_path, &tmpl_path) {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::error!(
                            "Error watch application.yaml: path({:#?}) {:#?}",
                            application_path,
                            e
                        );
                        continue;
                    }
                };
            let app_tmpl = Arc::new(AppTmplState {
                tmpl_service: ServeDir::new(&tmpl_path),
                id: id.to_string(),
                info: application_tmpl,
                tmpls,
                watcher: app_watcher,
                root_path: entry.as_path().to_path_buf(),
            });

            // insert app tmpl
            app_tmpls.insert(id.to_string(), app_tmpl);

            let id_clone = id.to_string();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = app_notifier.changed() => {
                            let content = match tokio::fs::read_to_string(&application_path).await {
                                Ok(f) => f,
                                Err(_) => continue,
                            };
                            let application_tmpl: ApplicationTmpl = match serde_yaml::from_str(&content) {
                                Ok(f) => f,
                                Err(e) => {
                                    error!("Error loading application.yaml: {}", e);
                                    continue;
                                }
                            };

                            {
                                let mut apps = get_app_tmpl_manager().app_tmpl_list.write().await;
                                if let Some(tmp) = apps.get_mut(&id_clone) {
                                    if let Some(tmp) = Arc::get_mut(tmp) {
                                        tmp.info = Some(application_tmpl);
                                    }
                                }
                            }
                        }
                    }
                }
            });
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
pub struct AppDeployInfo {
    pub name: String,
    pub node_name: String,
    pub values: Option<String>,
    pub project: Option<String>,
    pub build: Option<bool>,
}

pub async fn deploy(
    req: &AppDeployRequest,
    app_tmpl: &AppTmplState,
    log_writer: &tokio::sync::mpsc::UnboundedSender<String>,
) -> anyhow::Result<()> {
    let name = &req.name;
    let node_name = &req.node_name;
    let values = req.values.as_deref();
    let repositry = db::repository().await;
    let db_app = repositry
        .kvs
        .select_one("app", Some(name), None, None)
        .await?;
    let (insert, mut info): (bool, AppDeployInfo) = match db_app {
        Some(info) => (false, serde_json::from_str(&info.value)?),
        None => (
            true,
            AppDeployInfo {
                name: name.to_string(),
                node_name: node_name.to_string(),
                values: values.map(|v| v.to_string()),
                project: req.project.clone(),
                build: req.build,
            },
        ),
    };

    // deploy
    // 1. stop old app
    if !insert && &info.node_name != node_name {
        // get old app
        let old_node = node_manager().get_node(&info.node_name).await?;
        if let Some(old_node) = old_node {
            let options = Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            });
            let _ = &old_node.docker.remove_container(name, options).await;
        }
        let _ = log_writer.send(format!("stop app {} on node {}", name, info.node_name));
    }
    // 2. prepare new app, copy files to node
    let ctx: serde_yaml::Value = match values {
        Some(v) => serde_yaml::from_str(&v)?,
        None => serde_yaml::Value::Null,
    };

    let mut maps = HashMap::new();
    for tmpl in app_tmpl.tmpls.iter() {
        let context = app_tmpl.watcher.get_context(tmpl, &ctx).await?;
        maps.insert(tmpl.clone(), context);
    }

    // 3. start new app
    let new_node = node_manager().get_node(node_name).await?;
    let mut cli_args = Vec::new();
    let project_dir = match &req.project {
        Some(p) => Some(p.to_string()),
        None => {
            let project_dir = app_tmpl.get_default_project_path();
            if project_dir.exists() {
                project_dir.to_str().map(|d| d.to_string())
            } else {
                None
            }
        }
    };

    cli_args.push("-f");
    cli_args.push("-");
    cli_args.push("up");
    cli_args.push("-d");

    if let Some(true) = req.build {
        cli_args.push("--build");
    }

    if let Some(new_node) = new_node {
        let mut docker_compose_cli = DockerComposeCli::new(
            new_node.get_node_ip(),
            new_node.get_node_port(),
            &cli_args,
            project_dir,
        )?;
        if let Some(c) = get_docker_compose_file(&maps) {
            docker_compose_cli.run_cache(c).await?;
        }
    }

    let _ = log_writer.send(format!(
        "deploy app {} on node {} success",
        name, info.node_name
    ));

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
        info.node_name = node_name.to_string();
        info.values = values.map(|v| v.to_string());
        info.project = req.project.clone();
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

fn get_docker_compose_file(map: &HashMap<String, String>) -> Option<&str> {
    let tmp = map
        .iter()
        .find(|(k, v)| k.starts_with("docker-compose") && !v.is_empty())
        .map(|x| x.1.as_str());
    tracing::debug!("get_docker_compose_file: {:#?}", tmp);
    tmp
}
