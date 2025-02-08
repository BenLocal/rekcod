use std::{collections::HashMap, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use rekcod_core::{
    api::{req::RegisterNodeRequest, resp::NodeItemResponse},
    auth::get_token,
    constants::REKCOD_AGENT_PREFIX_PATH,
    docker::rekcod_connect,
    obj::NodeStatus,
};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, time::Instant};
use tracing::{info, warn};

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
    last_heartbeat: Instant,
}

impl NodeState {
    fn create(node: KvsForDb) -> anyhow::Result<Arc<Self>> {
        let node = Node::try_from(node)?;
        let docker_client = rekcod_connect(
            Some(format!("http://{}:{}", node.ip, node.port)),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            get_token(),
        )?;

        let state = Arc::new(Self {
            node,
            docker: docker_client,
            last_heartbeat: Instant::now(),
        });

        Ok(state)
    }

    pub fn get_node_ip(&self) -> &str {
        &self.node.ip
    }

    pub fn get_node_port(&self) -> u16 {
        self.node.port
    }

    pub fn get_last_heartbeat(&self) -> &Instant {
        &self.last_heartbeat
    }

    pub fn online(&self) -> bool {
        self.node.status
    }

    pub fn refresh_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    #[allow(dead_code)]
    fn get_node_host(&self) -> String {
        format!("http://{}:{}", self.node.ip, self.node.port)
    }

    #[allow(dead_code)]
    pub fn get_node_agent(&self) -> String {
        format!("{}{}", self.get_node_host(), REKCOD_AGENT_PREFIX_PATH)
    }
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
            let state = NodeState::create(node)?;
            let mut nodes = self.nodes.write().await;
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

    pub async fn get_all_nodes(&self, all: bool) -> anyhow::Result<Vec<Arc<NodeState>>> {
        let subkey = if all {
            None
        } else {
            Some(NodeStatus::Online.to_string())
        };

        // get from db
        let db_node = db::repository()
            .await
            .kvs
            .select("node", None, subkey.as_deref(), None)
            .await?;

        if !db_node.is_empty() {
            let mut nodes = self.nodes.write().await;
            for kv in db_node {
                match nodes.get_mut(&kv.key) {
                    Some(state) => {
                        let node = Node::try_from(kv)?;
                        if let Some(state) = Arc::get_mut(state) {
                            state.node = node;
                        }
                    }
                    None => {
                        let state = NodeState::create(kv)?;
                        nodes.insert(state.node.name.to_owned(), Arc::clone(&state));
                    }
                }
            }
            return Ok(nodes.values().cloned().collect());
        }

        // get from db is empty, clear cache
        {
            self.nodes.write().await.clear();
        }
        Ok(vec![])
    }

    pub async fn monitor_nodes(&self) -> anyhow::Result<()> {
        let mut nodes = node_manager().nodes.write().await;
        let now = Instant::now();
        for (name, state) in nodes.iter_mut() {
            let last = state.get_last_heartbeat();
            let since = now.duration_since(*last);
            info!(
                "Node {} last heartbeat {:?}, since: {:?}",
                name, last, since
            );
            if since > Duration::from_secs(15) {
                if state.online() {
                    // inactive node
                    warn!("Node {} is inactive", name);
                    self.update_node_status_db(name, false).await?;
                    if let Some(state) = Arc::get_mut(state) {
                        state.node.status = false;
                    }
                }
            } else {
                // active node
                if !state.online() {
                    info!("Node {} is active", name);
                    self.update_node_status_db(name, true).await?;
                    if let Some(state) = Arc::get_mut(state) {
                        state.node.status = true;
                    }
                }
            }
        }

        Ok(())
    }

    async fn update_node_status_db(&self, node_name: &str, status: bool) -> anyhow::Result<()> {
        let repositry = db::repository().await;
        let node = repositry
            .kvs
            .select_one("node", Some(node_name), None, None)
            .await?;

        // update node info
        if let Some(node) = node {
            let mut value_tmp: Node = serde_json::from_str(&node.value)?;
            value_tmp.status = status;
            let value = serde_json::to_string(&value_tmp)?;
            let status_node = if status {
                NodeStatus::Online
            } else {
                NodeStatus::Offline
            };
            repositry
                .kvs
                .update_value(
                    "node",
                    node_name,
                    Some(&status_node.to_string()),
                    None,
                    &value,
                )
                .await?;
        }

        Ok(())
    }

    pub async fn refresh_node_heartbeat(&self, node_name: &str) -> anyhow::Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_name) {
            if let Some(node) = Arc::get_mut(node) {
                node.refresh_heartbeat();
            }
        }
        Ok(())
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
        let mut node: Node = serde_json::from_str(&kvs.value)?;
        // modify name from key
        node.name = kvs.key;
        // modify status from sub_key
        match NodeStatus::from(&kvs.sub_key) {
            NodeStatus::Online => node.status = true,
            NodeStatus::Offline => node.status = false,
        };
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

impl Into<NodeItemResponse> for Node {
    fn into(self) -> NodeItemResponse {
        NodeItemResponse {
            name: self.name,
            host_name: self.host_name,
            ip: self.ip,
            port: self.port,
            version: self.version,
            arch: self.arch,
            os: self.os,
            os_version: self.os_version,
            os_kernel: self.os_kernel,
            status: self.status,
        }
    }
}
