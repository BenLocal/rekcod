use std::{path::PathBuf, sync::Arc};

use bollard::{container::InspectContainerOptions, secret::ContainerInspectResponse};
use minijinja::{
    context, path_loader,
    syntax::SyntaxConfig,
    value::{from_args, Object},
    Environment, Error, State,
};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

use crate::{env::env_manager, node::manager::node_manager};

pub struct Engine {
    #[allow(dead_code)]
    reloader: AutoReloader,
}

impl Engine {
    pub fn new(template_path: &PathBuf) -> Engine {
        let template_path = template_path.to_owned();
        let reloader = AutoReloader::new(move |notifier| {
            let mut env = Environment::new();
            let syntax = get_syntax(&template_path);
            if let Some(syntax) = syntax {
                env.set_syntax(syntax);
            }
            env.add_filter("default", none_default);
            env.set_loader(path_loader(&template_path));
            notifier.set_fast_reload(true);
            notifier.watch_path(&template_path, true);
            Ok(env)
        });

        Engine { reloader: reloader }
    }

    pub async fn render<S: Serialize>(
        &self,
        template_name: &str,
        value: S,
    ) -> anyhow::Result<String> {
        let env = self.reloader.acquire_env()?;
        let tmpl = env.get_template(template_name)?;
        let ec = create_context(value);
        tmpl.render(ec).map_err(|err| err.into())
    }
}

fn get_syntax(template_path: &PathBuf) -> Option<SyntaxConfig> {
    let file_name = template_path.file_name()?.to_string_lossy().to_string();

    let mut name = file_name.as_str();
    if file_name.ends_with(".j2") {
        name = &name[..file_name.len() - 3];
    } else if file_name.ends_with(".jinja") {
        name = &name[..file_name.len() - 6];
    }

    if name.ends_with(".yaml") || name.ends_with(".yml") {
        SyntaxConfig::builder()
            .line_comment_prefix("#")
            .build()
            .ok()
    } else if name.ends_with(".json") {
        SyntaxConfig::builder()
            .line_comment_prefix("//")
            .build()
            .ok()
    } else {
        None
    }
}

pub async fn render_dynamic_tmpl<S: Serialize>(
    template_content: &str,
    value: S,
) -> anyhow::Result<String> {
    let ec = create_context(value);
    Ok(Environment::new().render_str(template_content, ec)?)
}

fn none_default(
    value: minijinja::value::Value,
    other: Option<minijinja::value::Value>,
) -> minijinja::value::Value {
    if value.is_undefined() || value.is_none() {
        other.unwrap_or_else(|| minijinja::value::Value::from(""))
    } else {
        value
    }
}

fn create_context<S: Serialize>(value: S) -> minijinja::value::Value {
    context! {
        Docker => minijinja::value::Value::from_object(DockerContext),
        Value => value,
        Env => minijinja::value::Value::from_object(EnvironmentContext)
    }
}

#[derive(Debug)]
struct EnvironmentContext;

impl EnvironmentContext {
    async fn get_env_value(self: Arc<Self>, key: &str) -> Option<minijinja::value::Value> {
        let v = env_manager().get(key).await;
        match v {
            Some(v) => Some(minijinja::value::Value::from(v)),
            None => None,
        }
    }
}

impl Object for EnvironmentContext {
    fn get_value(self: &Arc<Self>, key: &minijinja::Value) -> Option<minijinja::Value> {
        Some(match key.as_str()? {
            k => minijinja::value::Value::from(tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.clone().get_env_value(k))
            })),
        })
    }
}

#[derive(Debug)]
struct DockerContext;

impl DockerContext {
    async fn ps_inspect(self: Arc<Self>, key: Arc<str>) -> Option<minijinja::value::Value> {
        let nodes = node_manager().get_all_nodes(false).await.unwrap();

        for node in nodes {
            let options = Some(InspectContainerOptions { size: false });
            let resp = match node.docker.inspect_container(&key, options).await {
                Ok(resp) => resp,
                Err(_) => continue,
            };

            return Some(context! {
                Data => ContainerInspectResponse::from(resp),
                Node => node.node.name,
            });
        }

        None
    }
}

impl Object for DockerContext {
    fn call_method(
        self: &Arc<Self>,
        _state: &State<'_, '_>,
        method: &str,
        args: &[minijinja::value::Value],
    ) -> Result<minijinja::value::Value, Error> {
        let (key,) = from_args(args)?;
        match method {
            "ps_inspect" => Ok(minijinja::value::Value::from(tokio::task::block_in_place(
                || tokio::runtime::Handle::current().block_on(self.clone().ps_inspect(key)),
            ))),
            _ => Err(Error::from(minijinja::ErrorKind::UnknownMethod)),
        }
    }
}
