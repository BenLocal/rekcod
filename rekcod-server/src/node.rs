use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;
use rekcod_core::{api::req::RegisterNodeRequest, docker::rekcod_connect};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::db::{self, kvs::KvsForDb};

static NODE_MANAGER: Lazy<NodeManager> = Lazy::new(NodeManager::new);

pub fn node_manager() -> &'static NodeManager {
    &NODE_MANAGER
}

pub struct NodeManager {
    nodes: RwLock<HashMap<String, Arc<NodeState>>>,
}

#[derive(Debug)]
pub struct NodeState {
    pub node: Node,
    pub docker: bollard::Docker,
}

impl NodeManager {
    fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_node(&self, name: &str) -> anyhow::Result<Option<Arc<NodeState>>> {
        {
            let nodes = self.nodes.read().await;
            let tmp = nodes.get(name);
            if let Some(tmp) = tmp {
                return Ok(Some(Arc::clone(&tmp)));
            }
        }

        // get from db
        let repositry = db::repository().await;
        let node = repositry
            .kvs
            .select_one("node", Some(name), None, None)
            .await?;
        if let Some(node) = node {
            let mut nodes = self.nodes.write().await;
            let node = Node::try_from(node)?;
            let docker_client = rekcod_connect(
                Some(format!("http://{}:{}", node.ip, node.port)),
                rekcod_core::constants::DOCKER_PROXY_PATH,
                4,
            )?;

            let state = Arc::new(NodeState {
                node,
                docker: docker_client,
            });
            nodes.insert(name.to_owned(), Arc::clone(&state));
            return Ok(Some(state));
        }

        Ok(None)
    }

    pub async fn delete_node(&self, node_name: &str) -> anyhow::Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_name);
        Ok(())
    }

    pub async fn get_all_nodes(&self) -> anyhow::Result<Vec<Arc<NodeState>>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.values().cloned().collect())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct Node {
    pub name: String,
    pub host_name: String,
    pub ip: String,
    pub port: u16,
    pub token: String,
    pub version: String,
    pub arch: String,
    pub os: String,
    pub os_version: String,
    pub os_kernel: String,
    pub status: bool,
}

impl TryFrom<KvsForDb> for Node {
    type Error = anyhow::Error;

    fn try_from(kvs: KvsForDb) -> Result<Self, Self::Error> {
        let node: Node = serde_json::from_str(&kvs.value)?;
        Ok(node)
    }
}

impl TryFrom<RegisterNodeRequest> for Node {
    type Error = anyhow::Error;

    fn try_from(req: RegisterNodeRequest) -> Result<Self, Self::Error> {
        let node = Node {
            name: req.name,
            host_name: req.host_name,
            ip: req.ip,
            port: req.port,
            token: req.token,
            version: req.version,
            arch: req.arch,
            os: req.os,
            os_version: req.os_version,
            os_kernel: req.os_kernel,
            status: req.status,
        };
        Ok(node)
    }
}
