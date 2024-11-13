use std::{path::PathBuf, sync::Arc};

use bollard::{container::InspectContainerOptions, secret::ContainerInspectResponse};
use minijinja::{
    context, path_loader,
    value::{from_args, Object},
    Environment, Error, State,
};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;
use tokio::runtime::Handle;

use crate::node::node_manager;

pub struct Engine {
    #[allow(dead_code)]
    reloader: AutoReloader,
}

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

        Engine { reloader: reloader }
    }

    pub fn render<S: Serialize>(
        &self,
        template_name: &str,
        value: S,
        rt: tokio::runtime::Handle,
    ) -> anyhow::Result<String> {
        let env = self.reloader.acquire_env()?;
        let tmpl = env.get_template(template_name)?;
        let ec = context! {
            Docker => minijinja::value::Value::from_object(DockerContext::new(rt)),
            Value => value
        };
        tmpl.render(ec).map_err(|err| err.into())
    }
}

pub fn render_dynamic_tmpl<S: Serialize>(
    template_content: &str,
    value: S,
    rt: tokio::runtime::Handle,
) -> anyhow::Result<String> {
    let ec = context! {
        Docker => minijinja::value::Value::from_object(DockerContext::new(rt)),
        Value => value
    };

    Ok(Environment::new().render_str(template_content, ec)?)
}

#[derive(Debug)]
struct DockerContext {
    rt: tokio::runtime::Handle,
}

impl DockerContext {
    fn new(rt: tokio::runtime::Handle) -> DockerContext {
        DockerContext { rt: rt }
    }

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
            "ps_inspect" => Ok(minijinja::value::Value::from(
                self.rt.block_on(self.clone().ps_inspect(key)),
            )),
            _ => Err(Error::from(minijinja::ErrorKind::UnknownMethod)),
        }
    }
}
