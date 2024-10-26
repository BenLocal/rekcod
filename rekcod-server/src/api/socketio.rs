use std::{collections::HashMap, sync::Arc};

use bollard::container::{AttachContainerOptions, AttachContainerResults};
use futures::StreamExt;
use socketioxide::{
    extract::{Data, SocketRef},
    layer::SocketIoLayer,
    SocketIo,
};
use sqlx::error;
use tokio::{io::AsyncWriteExt, sync::Mutex};
use tracing::info;
use url::Url;

use crate::node::node_manager;

pub fn socketio_routers() -> SocketIoLayer {
    let (layer, io) = SocketIo::new_layer();
    io.ns("/node/docker/container/attch", on_connect);
    layer
}

async fn on_connect(socket: SocketRef) {
    info!(ns = socket.ns(), ?socket.id, "Socket.IO connected");
    let res = match connect_to_docker(&socket).await {
        Ok(Some(data)) => {
            socket.emit("connected", "ok").ok();
            data
        }
        Ok(None) => {
            // do nothing
            socket.emit("connected", "can not connect to docker").ok();
            return;
        }
        Err(err) => {
            // do nothing
            socket.emit("connected", &err.to_string()).ok();
            return;
        }
    };

    let write = Arc::new(Mutex::new(res.input));
    let write_clone = Arc::clone(&write);
    let read = Arc::new(Mutex::new(res.output));
    socket.on(
        "data",
        |socket: SocketRef, Data::<String>(data)| async move {
            info!(?data, "Received event:");
            {
                let mut write_lock = write.lock().await;
                if let Err(err) = write_lock.write(data.as_bytes()).await {
                    info!(?err, "Failed to write data");
                }
                if let Err(err) = write_lock.flush().await {
                    info!(?err, "Failed to flush data");
                }
            }
            {
                let mut read_lock = read.lock().await;
                while let Some(data) = read_lock.next().await {
                    if let Ok(data) = data {
                        socket.emit("data", data.as_ref()).ok();
                    }
                }
            }
        },
    );

    socket.on_disconnect(|s: SocketRef| async move {
        info!(ns = s.ns(), ?s.id, "Socket.IO disconnected");
        // do nothing
        s.emit("disconnected", "ok").ok();
    });
}

async fn connect_to_docker(socket: &SocketRef) -> anyhow::Result<Option<AttachContainerResults>> {
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

    // get node
    let n = node_manager().get_node(&node_name.unwrap()).await?;
    if let Some(n) = n {
        let options = Some(AttachContainerOptions::<String> {
            stdin: Some(true),
            stdout: Some(true),
            stderr: Some(true),
            stream: Some(true),
            logs: Some(true),
            detach_keys: Some("ctrl-c".to_string()),
        });
        let docker_client = &n.docker;
        let res = docker_client
            .attach_container(&id.unwrap(), options)
            .await?;
        return Ok(Some(res));
    }

    Ok(None)
}

#[cfg(test)]
mod test {
    use bollard::{
        container::{AttachContainerOptions, LogOutput},
        exec::{CreateExecOptions, StartExecOptions, StartExecResults},
        Docker,
    };
    use futures::StreamExt;
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

    #[tokio::test]
    async fn test_exec_container() -> anyhow::Result<()> {
        let docker = Docker::connect_with_defaults()?;
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
}
