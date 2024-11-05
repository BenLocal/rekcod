use std::{collections::HashMap, sync::Arc};

use bollard::{
    container::LogOutput,
    exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef},
    layer::SocketIoLayer,
    SocketIo,
};
use tokio::{io::AsyncWriteExt, sync::Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use url::Url;

use crate::node::{node_manager, NodeState};

pub fn socketio_routers() -> SocketIoLayer {
    let (layer, io) = SocketIo::new_layer();
    io.ns("/api/node/docker/container/exec", on_connect);
    layer
}

async fn on_connect(socket: SocketRef) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO connected");
    let (node_name, id) = match get_query_params(&socket).await {
        Ok((node_name, id)) => (node_name, id),
        Err(err) => {
            socket.emit("err", &err.to_string()).ok();
            return;
        }
    };

    let node = match node_manager().get_node(&node_name).await {
        Ok(Some(n)) => n,
        Ok(None) => {
            socket.emit("err", "node not found").ok();
            return;
        }
        Err(err) => {
            socket.emit("err", &err.to_string()).ok();
            return;
        }
    };

    let node_clone = Arc::clone(&node);
    let (exec_id, res) = match connect_to_docker(node_clone, &id).await {
        Ok(data) => {
            socket.emit("connected", "ok").ok();
            data
        }
        Err(err) => {
            // do nothing
            socket.emit("err", &err.to_string()).ok();
            return;
        }
    };

    let (input, mut output) = match res {
        StartExecResults::Attached { output, input } => (input, output),
        _ => {
            // do nothing
            socket.emit("err", "can not connect to docker").ok();
            return;
        }
    };

    let input = Arc::new(Mutex::new(input));
    let disconnect_input = Arc::clone(&input);
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    socket.on(
        "data",
        |_socket: SocketRef, Data::<String>(data)| async move {
            debug!(?data, "Received event:");
            {
                let mut input = input.lock().await;
                if let Err(err) = input.write(data.as_bytes()).await {
                    info!(?err, "Failed to write data");
                }
                if let Err(err) = input.flush().await {
                    info!(?err, "Failed to flush data");
                }
            }
        },
    );

    let node_clone = Arc::clone(&node);
    socket.on(
        "resize",
        |_socket: SocketRef, Data::<ResizeInfo>(data)| async move {
            debug!(?data, "Received resize event:");
            {
                let _ = resize_docker_cmd(node_clone, &data, &exec_id).await;
            }
        },
    );

    socket.on_disconnect(|s: SocketRef| async move {
        info!(ns = s.ns(), ?s.id, "Socket.IO disconnected");
        let mut w = disconnect_input.lock().await;
        let _ = w.write(b"exit\n").await;
        let _ = w.flush().await;
        cancel.cancel();
        s.emit("disconnected", "ok").ok();
    });

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    break;
                }

                Some(res) = output.next() => {
                    if let Ok(res) = res {
                        match res {
                            LogOutput::StdOut { message } => {
                                let s = String::from_utf8_lossy(&message).to_string();
                                let _ = socket.emit("out", &s);
                            }
                            LogOutput::StdErr { message } => {
                                let s = String::from_utf8_lossy(&message).to_string();
                                let _ = socket.emit("out", &s);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

async fn connect_to_docker(
    node: Arc<NodeState>,
    container_id: &str,
) -> anyhow::Result<(String, StartExecResults)> {
    // get node
    let config = CreateExecOptions {
        cmd: Some(vec!["sh"]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        tty: Some(true),
        ..Default::default()
    };
    let s = &node.docker.create_exec(&container_id, config).await?;
    let res = node
        .docker
        .start_exec(&s.id, None::<StartExecOptions>)
        .await?;
    return Ok((s.id.clone(), res));
}

async fn get_query_params(socket: &SocketRef) -> anyhow::Result<(String, String)> {
    let base_url = "http://example.com";
    let req_parts = socket.req_parts();
    let full_url = format!("{}{}", base_url, &req_parts.uri.to_string());
    let url = Url::parse(&full_url)?;
    let query_params = url.query_pairs().into_owned().collect::<HashMap<_, _>>();

    let node_name = query_params.get("node_name");
    let id = query_params.get("id");

    if node_name.is_none() || id.is_none() {
        // params error
        return Err(anyhow::anyhow!("params error").into());
    }

    Ok((node_name.unwrap().to_string(), id.unwrap().to_string()))
}

async fn resize_docker_cmd(
    node: Arc<NodeState>,
    info: &ResizeInfo,
    message_id: &str,
) -> anyhow::Result<()> {
    let options = ResizeExecOptions {
        height: info.height as u16,
        width: info.width as u16,
        ..Default::default()
    };

    let _ = &node.docker.resize_exec(message_id, options).await?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResizeInfo {
    pub height: u32,
    pub width: u32,
}

#[cfg(test)]
mod test {
    use bollard::{
        container::{AttachContainerOptions, LogOutput},
        exec::{CreateExecOptions, StartExecOptions, StartExecResults},
        Docker,
    };
    use futures::StreamExt;
    use rekcod_core::docker::rekcod_connect;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_attch_container() -> anyhow::Result<()> {
        let docker = Docker::connect_with_defaults()?;

        let options = Some(AttachContainerOptions::<String> {
            stdin: Some(true),
            stdout: Some(true),
            stderr: Some(true),
            stream: Some(true),
            logs: Some(true),
            detach_keys: Some("ctrl-c".to_string()),
        });

        let mut res = docker.attach_container("6f3ee795d19a", options).await?;

        res.input.write(b"ls\n").await?;
        res.input.flush().await?;

        while let Some(res) = res.output.next().await {
            if let Ok(res) = res {
                match res {
                    LogOutput::StdOut { message } => {
                        let s = String::from_utf8_lossy(&message).to_string();
                        println!("{}", s);
                    }
                    LogOutput::StdErr { message } => {
                        let s = String::from_utf8_lossy(&message).to_string();
                        println!("{}", s);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn rekcod_exec_connect(docker: &Docker) -> anyhow::Result<()> {
        let config = CreateExecOptions {
            cmd: Some(vec!["sh"]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            attach_stdin: Some(true),
            // detach_keys: Some("exit"),
            ..Default::default()
        };
        let s = docker.create_exec("2d0", config).await?;
        let res = docker.start_exec(&s.id, None::<StartExecOptions>).await?;

        if let StartExecResults::Attached {
            mut output,
            mut input,
        } = res
        {
            tokio::spawn(async move {
                while let Some(res) = output.next().await {
                    if let Ok(res) = res {
                        match res {
                            LogOutput::StdOut { message } => {
                                let s = String::from_utf8_lossy(&message).to_string();
                                println!("{}", s);
                            }
                            LogOutput::StdErr { message } => {
                                let s = String::from_utf8_lossy(&message).to_string();
                                println!("{}", s);
                            }
                            _ => {}
                        }
                    }
                }

                println!("exit");
            });

            input.write(b"ls -lha\n").await?;
            input.flush().await?;

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let res = docker.inspect_exec(&s.id).await?;
            println!("{:?}", res);

            input.write(b"exit\n").await?;
            input.flush().await?;
            let res = docker.inspect_exec(&s.id).await?;
            println!("{:?}", res);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_exec_container() -> anyhow::Result<()> {
        let docker = Docker::connect_with_defaults()?;
        rekcod_exec_connect(&docker).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_exec_container_proxy() -> anyhow::Result<()> {
        let docker = rekcod_connect(
            Some("http://127.0.0.1:6734"),
            rekcod_core::constants::DOCKER_PROXY_PATH,
            40,
            "8ca8928c-a13a-4ebb-98d4-5e82e8fb096b",
        )?;
        rekcod_exec_connect(&docker).await?;
        Ok(())
    }
}
