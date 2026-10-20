use crate::config::{with_port, HelperConfig};
use anyhow::{bail, Context, Result};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader, Lines},
    process::{Child, Command},
    time::timeout,
};

pub struct Helper {
    _child: Child,
    pub port: u16,
}

impl Helper {
    pub async fn start(config: &HelperConfig, port: u16) -> Result<Self> {
        // A helper usually binds its port itself; checking first turns its own error into a
        // message that names the port and the way out.
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
            bail!(
                "port {port} is already in use, maybe by a helper left over from an earlier run; \
                 stop it or move the helpers with first_port"
            );
        }
        let mut command = Command::new(&config.command);
        command
            .args(with_port(&config.args, Some(port)))
            .envs(&config.env)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(dir) = &config.dir {
            command.current_dir(dir);
        }
        let mut child = command
            .spawn()
            .with_context(|| format!("starting {}", config.command.display()))?;
        let mut out = BufReader::new(child.stdout.take().context("helper stdout")?).lines();
        let mut err = BufReader::new(child.stderr.take().context("helper stderr")?).lines();
        let mut tail: Vec<String> = Vec::new();
        let ready = timeout(config.ready_timeout(), async {
            let (mut out_open, mut err_open) = (true, true);
            while out_open || err_open {
                let line = tokio::select! {
                    line = out.next_line(), if out_open => line?.or_else(|| { out_open = false; None }),
                    line = err.next_line(), if err_open => line?.or_else(|| { err_open = false; None }),
                };
                let Some(line) = line else { continue };
                tracing::debug!(port, "{line}");
                if line.contains(&config.ready) {
                    return Ok(true);
                }
                tail.push(line);
                if tail.len() > 20 {
                    tail.remove(0);
                }
            }
            Ok::<_, std::io::Error>(false)
        })
        .await;
        match ready {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => bail!(
                "helper on port {port} exited before it was ready:\n{}",
                tail.join("\n")
            ),
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => bail!(
                "helper on port {port} not ready after {:?}",
                config.ready_timeout()
            ),
        }
        // Keep draining its output so a full pipe never blocks it.
        drain(out, port);
        drain(err, port);
        Ok(Helper {
            _child: child,
            port,
        })
    }
}

fn drain<R: AsyncRead + Unpin + Send + 'static>(mut lines: Lines<BufReader<R>>, port: u16) {
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(port, "{line}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> HelperConfig {
        HelperConfig {
            command: "sh".into(),
            args: vec!["-c".into(), script.into()],
            dir: None,
            env: Default::default(),
            ready: "READY".into(),
            ready_timeout_secs: 5,
            count: 1,
            first_port: 0,
        }
    }

    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn a_helper_is_ready_when_it_says_so_on_either_stream() {
        for script in [
            "echo loading; echo READY on {port}; exec sleep 30",
            "echo loading >&2; echo READY >&2; exec sleep 30",
        ] {
            let port = free_port();
            let helper = Helper::start(&shell(script), port).await.unwrap();
            assert_eq!(helper.port, port);
        }
    }

    #[tokio::test]
    async fn a_helper_that_dies_first_says_what_it_printed() {
        let error = Helper::start(&shell("echo no weights here >&2; exit 1"), free_port())
            .await
            .err()
            .unwrap();
        assert!(
            format!("{error:#}").contains("no weights here"),
            "{error:#}"
        );
    }
}
