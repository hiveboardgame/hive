mod api;
mod engine;
mod evaluate;
mod scheme;
mod servers;

use anyhow::{Context, Result};
use api::{Api, Ownership};
use clap::Parser;
use engine::Engine;
use evaluate::{evaluate, Progress};
use scheme::Scheme;
use servers::{EvalServer, ServerSetup};
use sha2::{Digest, Sha256};
use shared_types::{EvalJob, EvalResult};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

/// Takes games from hivegame.com's eval queue, searches them with StockBee and posts back
/// which moves were inaccuracies, mistakes and blunders.
#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env = "HIVE_URL", default_value = "http://localhost:3000")]
    base_url: String,
    #[arg(long, env = "EVAL_WORKER_TOKEN", hide_env_values = true)]
    token: String,
    /// Name this worker reports under; must be unique among running workers.
    #[arg(long, env = "EVAL_WORKER_NAME")]
    worker: Option<String>,
    /// Directory holding StockBee's tools/ and build/.
    #[arg(long, env = "STOCKBEE_DIR", default_value = "stockbee")]
    stockbee_dir: PathBuf,
    #[arg(long, env = "STOCKBEE_NET", default_value = "stockbee.pt")]
    net: PathBuf,
    #[arg(long, env = "STOCKBEE_PYTHON", default_value = "python3")]
    python: PathBuf,
    /// cpu or cuda.
    #[arg(long, default_value = "cpu")]
    device: String,
    /// Eval servers to run; each gets its own engines and torch threads. On the 4-core prod
    /// box three servers with two threads each beat one server with six threads.
    #[arg(long, default_value_t = 3)]
    servers: u16,
    #[arg(long, default_value_t = 1)]
    engines_per_server: u16,
    #[arg(long, default_value_t = 2)]
    torch_threads: u32,
    /// First local port for the eval servers; each further server takes the next one.
    #[arg(long, default_value_t = 41871)]
    base_port: u16,
    /// Also take games the site picks itself when no user is waiting; `EVAL_AUTO=false` keeps
    /// the worker to user requests, e.g. for a gentle first start.
    #[arg(long, env = "EVAL_AUTO", default_value_t = true, action = clap::ArgAction::Set)]
    auto: bool,
    #[arg(long, default_value_t = 64)]
    screen_sims: u32,
    #[arg(long, default_value_t = 2.0)]
    threshold: f32,
    #[arg(long, default_value_t = 800)]
    full_sims: u32,
}

const IDLE_POLL: Duration = Duration::from_secs(5);
const ERROR_BACKOFF: Duration = Duration::from_secs(30);
const PROGRESS_EVERY: Duration = Duration::from_secs(5);

struct Workers {
    _servers: Vec<EvalServer>,
    engines: Vec<Engine>,
}

impl Workers {
    async fn start(args: &Args, setup: &ServerSetup) -> Result<Self> {
        let mut servers = Vec::new();
        let mut engines = Vec::new();
        let binary = args.stockbee_dir.join("build").join("stockbee");
        for i in 0..args.servers {
            let server = EvalServer::start(setup, args.base_port + i).await?;
            for _ in 0..args.engines_per_server {
                engines.push(Engine::start(&binary, server.port).await?);
            }
            servers.push(server);
        }
        tracing::info!(
            servers = servers.len(),
            engines = engines.len(),
            "workers ready"
        );
        Ok(Workers {
            _servers: servers,
            engines,
        })
    }
}

fn net_digest(path: &PathBuf) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().take(6).map(|b| format!("{b:02x}")).collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Dropping `run` on a signal drops the eval servers and engines, which kills them; a plain
    // SIGTERM would otherwise leave the Python servers running and holding their ports.
    tokio::select! {
        result = run() => result,
        _ = shutdown_signal() => {
            tracing::info!("shutting down");
            Ok(())
        }
    }
}

async fn shutdown_signal() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("installing the SIGTERM handler");
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    let worker = args
        .worker
        .clone()
        .unwrap_or_else(|| format!("evaluator-{}", std::process::id()));
    // The server runs from inside the StockBee directory, so it needs an absolute path.
    let net = std::fs::canonicalize(args.stockbee_dir.join(&args.net))
        .with_context(|| format!("finding the net {}", args.net.display()))?;
    let setup = ServerSetup {
        python: args.python.clone(),
        stockbee_dir: args.stockbee_dir.clone(),
        net: net.clone(),
        device: args.device.clone(),
        torch_threads: args.torch_threads,
    };
    let scheme = Scheme {
        screen: args.screen_sims,
        threshold: args.threshold,
        full: args.full_sims,
    };
    let mut api = Api::new(&args.base_url, &args.token, &worker);
    api.auto = args.auto;
    let digest = net_digest(&net)?;

    let mut workers = Workers::start(&args, &setup).await?;
    let engine_id = format!(
        "{} net {digest} {}",
        workers
            .engines
            .first()
            .map(|e| e.id.as_str())
            .unwrap_or("unknown"),
        scheme.label()
    );
    tracing::info!(%worker, %engine_id, auto = args.auto, "waiting for evals");

    loop {
        let job = match api.claim().await {
            Ok(Some(job)) => job,
            Ok(None) => {
                tokio::time::sleep(IDLE_POLL).await;
                continue;
            }
            Err(e) => {
                tracing::warn!("claim failed: {e:#}");
                tokio::time::sleep(ERROR_BACKOFF).await;
                continue;
            }
        };
        let healthy = run_job(&api, job, &mut workers, &scheme, &engine_id).await;
        if !healthy {
            tracing::warn!("restarting eval servers and engines");
            drop(std::mem::replace(
                &mut workers,
                Workers {
                    _servers: Vec::new(),
                    engines: Vec::new(),
                },
            ));
            workers = loop {
                match Workers::start(&args, &setup).await {
                    Ok(w) => break w,
                    Err(e) => {
                        tracing::error!("restart failed: {e:#}");
                        tokio::time::sleep(ERROR_BACKOFF).await;
                    }
                }
            };
        }
    }
}

/// Runs one eval to completion. Returns false when the engines need restarting.
async fn run_job(
    api: &Api,
    job: EvalJob,
    workers: &mut Workers,
    scheme: &Scheme,
    engine_id: &str,
) -> bool {
    let eval_id = job.eval_id;
    let started = std::time::Instant::now();
    tracing::info!(%eval_id, moves = job.moves.len(), "evaluating");

    let progress = Arc::new(Progress::default());
    let reporter = {
        let (api, progress) = (api.clone(), progress.clone());
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(PROGRESS_EVERY);
            loop {
                tick.tick().await;
                match api.progress(eval_id, progress.percent()).await {
                    Ok(Ownership::Lost) => {
                        tracing::warn!(%eval_id, "eval was taken away; stopping");
                        progress.cancelled.store(true, Ordering::Relaxed);
                        return;
                    }
                    Ok(Ownership::Ours) => {}
                    Err(e) => tracing::warn!(%eval_id, "progress report failed: {e:#}"),
                }
            }
        })
    };

    let engines = std::mem::take(&mut workers.engines);
    let (engines, outcome) = evaluate(engines, &job.game_type, job.moves, scheme, &progress).await;
    workers.engines = engines;
    reporter.abort();

    let lost = progress.cancelled.load(Ordering::Relaxed);
    match outcome {
        Ok(moves) if !lost => {
            match api.submit(eval_id, engine_id, EvalResult { moves }).await {
                Ok(Ownership::Ours) => {
                    tracing::info!(%eval_id, secs = started.elapsed().as_secs(), "done")
                }
                Ok(Ownership::Lost) => {
                    tracing::warn!(%eval_id, "finished an eval that was no longer ours")
                }
                Err(e) => tracing::error!(%eval_id, "submitting failed: {e:#}"),
            }
            true
        }
        // The server took the eval back; nothing to report and the engines are fine.
        _ if lost => true,
        Err(e) => {
            tracing::error!(%eval_id, "eval failed: {e:#}");
            if let Err(report) = api.fail(eval_id, &format!("{e:#}")).await {
                tracing::error!(%eval_id, "reporting the failure failed: {report:#}");
            }
            false
        }
        Ok(_) => true,
    }
}
