use anyhow::{bail, Context, Result};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    time::timeout,
};

/// How to run StockBee's eval server (tools/az_eval_server.py), which holds the net.
#[derive(Debug, Clone)]
pub struct ServerSetup {
    pub python: PathBuf,
    pub stockbee_dir: PathBuf,
    pub net: PathBuf,
    pub device: String,
    pub torch_threads: u32,
}

/// A CUDA server compiles the net before it is ready; a CPU one is ready in seconds.
const READY_TIMEOUT: Duration = Duration::from_secs(900);

pub struct EvalServer {
    _child: Child,
    pub port: u16,
}

impl EvalServer {
    pub async fn start(setup: &ServerSetup, port: u16) -> Result<Self> {
        // The server binds 127.0.0.1 itself; checking first turns its traceback into a message
        // that names the port and the way out.
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
            bail!(
                "port {port} is already in use, maybe by an eval server left over from an \
                 earlier run; stop it or choose other ports with --base-port"
            );
        }
        let mut command = Command::new(&setup.python);
        command
            .current_dir(&setup.stockbee_dir)
            .arg("-u")
            .arg("tools/az_eval_server.py")
            .arg(&setup.net)
            .arg(std::env::temp_dir().join(format!("hive-evaluator-{port}")))
            .args(["--serve-port", &port.to_string(), "--serve-reload", "0"])
            .args(["--device", &setup.device])
            .env("SB_EVAL_THREADS", setup.torch_threads.to_string())
            // Fail at startup rather than silently fall back to the much slower numpy path.
            .env("SB_REQUIRE_NATIVE_GF", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if setup.device == "cpu" {
            command.arg("--no-compile-graph");
        }
        let mut child = command.spawn().context("starting az_eval_server.py")?;
        let mut log = BufReader::new(child.stderr.take().context("server stderr")?).lines();
        let mut tail: Vec<String> = Vec::new();
        let ready = timeout(READY_TIMEOUT, async {
            while let Some(line) = log.next_line().await? {
                tracing::debug!(port, "{line}");
                if line.contains("READY") {
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
                "eval server on port {port} exited before it was ready:\n{}",
                tail.join("\n")
            ),
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => bail!("eval server on port {port} not ready after {READY_TIMEOUT:?}"),
        }
        // Keep draining the log so a full pipe never blocks the server.
        tokio::spawn(async move {
            while let Ok(Some(line)) = log.next_line().await {
                tracing::debug!(port, "{line}");
            }
        });
        Ok(EvalServer {
            _child: child,
            port,
        })
    }
}
