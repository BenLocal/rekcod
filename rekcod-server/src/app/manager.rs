use std::collections::HashMap;

use rekcod_core::application::Application;

use super::watch::AppWatcher;

pub struct AppManager {
    pub app_list: HashMap<String, AppState>,
}

pub struct AppState {
    pub app: Application,
    pub watcher: AppWatcher,
}
