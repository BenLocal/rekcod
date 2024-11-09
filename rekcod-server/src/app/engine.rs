use std::path::PathBuf;

use minijinja::{path_loader, Environment};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

pub struct Engine(AutoReloader);

impl Engine {
    pub fn new(template_path: &PathBuf) -> Engine {
        let template_path = template_path.to_owned();
        let reloader = AutoReloader::new(move |notifier| {
            let mut env = Environment::new();
            env.set_loader(path_loader(&template_path));
            notifier.set_fast_reload(true);
            notifier.watch_path(&template_path, true);
            Ok(env)
        });

        Engine(reloader)
    }

    pub fn render<S: Serialize>(&self, template_name: &str, ctx: S) -> anyhow::Result<String> {
        let env = self.0.acquire_env()?;
        let tmpl = env.get_template(template_name)?;
        tmpl.render(ctx).map_err(|err| err.into())
    }
}

pub fn render_dynamic_tmpl<S: Serialize>(template_content: &str, ctx: S) -> anyhow::Result<String> {
    Ok(Environment::new().render_str(template_content, ctx)?)
}
