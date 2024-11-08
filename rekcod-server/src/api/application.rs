use std::path::PathBuf;

use axum::Json;
use rekcod_core::{
    api::resp::{ApiJsonResponse, ApplicationResponse},
    application::Application,
    http::ApiError,
};

use crate::config::rekcod_server_config;

pub async fn get_app_list() -> Result<Json<ApiJsonResponse<Vec<ApplicationResponse>>>, ApiError> {
    let config = rekcod_server_config();
    let (mut dir_entries, _) = walk_dir(&config.get_app_root_path()).await?;

    let mut apps = Vec::new();

    while let Some(entry) = dir_entries.pop() {
        let id = match entry.file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        let (_, files) = walk_dir(&entry.join("template")).await?;
        let tmpls = files
            .into_iter()
            .filter_map(|f| f.file_name().map(|d| d.to_string_lossy().to_string()))
            .collect::<Vec<_>>();

        let file = match std::fs::File::open(entry.join("application.yaml")) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let application: Application = serde_yaml::from_reader(file)?;
        let app = ApplicationResponse {
            name: application.name,
            description: application.description,
            tmpls: tmpls,
            id: id.to_string(),
            version: application.version,
        };
        apps.push(app);
    }

    Ok(ApiJsonResponse::success(apps).into())
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
