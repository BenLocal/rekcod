use super::engine::Engine;

pub struct AppWatcher {
    pub path: String,
    pub tmpl_engine: Engine,
}
