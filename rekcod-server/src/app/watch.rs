use std::path::PathBuf;

use notify::{Error, Event, RecommendedWatcher, RecursiveMode, Watcher as _};
use serde_yaml::Value;

use super::engine::Engine;

type AppNotifier = tokio::sync::watch::Receiver<()>;

#[cfg(any(target_os = "linux", target_os = "android"))]
type AppWatcherType = notify::INotifyWatcher;
#[cfg(target_os = "macos")]
type AppWatcherType = notify::FsEventWatcher;

pub struct AppWatcher {
    _app_watcher: AppWatcherType,
    pub tmpl_engine: Engine,
}

impl AppWatcher {
    pub fn new(app_path: &PathBuf, tmpl_path: &PathBuf) -> anyhow::Result<(Self, AppNotifier)> {
        let tmpl_engine = Engine::new(tmpl_path);
        let (w, n) = Self::watch(&app_path)?;
        Ok((
            AppWatcher {
                _app_watcher: w,
                tmpl_engine,
            },
            n,
        ))
    }

    pub fn watch(
        path: &PathBuf,
    ) -> anyhow::Result<(AppWatcherType, tokio::sync::watch::Receiver<()>)> {
        let (tx, rx) = tokio::sync::watch::channel(());
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, Error>| {
                if let Ok(event) = result {
                    if event.kind.is_modify() {
                        let _ = tx.send(());
                    }
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&path, RecursiveMode::Recursive)?;
        Ok((watcher, rx))
    }

    pub fn get_context(
        &self,
        template_name: &str,
        ctx: &Value,
        rt: tokio::runtime::Handle,
    ) -> anyhow::Result<String> {
        self.tmpl_engine.render(template_name, ctx, rt)
    }
}
