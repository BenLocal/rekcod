use std::path::PathBuf;

use notify::{Error, Event, FsEventWatcher, RecommendedWatcher, RecursiveMode, Watcher as _};
use serde_yaml::Value;

use super::engine::Engine;

type AppNotifier = tokio::sync::watch::Receiver<()>;

pub struct AppWatcher {
    _app_watcher: FsEventWatcher,
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
    ) -> anyhow::Result<(FsEventWatcher, tokio::sync::watch::Receiver<()>)> {
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

    pub fn get_context(&self, template_name: &str, ctx: Value) -> anyhow::Result<String> {
        self.tmpl_engine.render(template_name, &ctx)
    }
}
