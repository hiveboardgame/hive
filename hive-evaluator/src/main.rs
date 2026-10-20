mod api;
mod config;
mod engine;
mod evaluate;
mod helper;
mod rules;
mod scheme;

use anyhow::{Context, Result};
use api::{worth_retrying, Api, Ownership};
use clap::Parser;
use config::EngineConfig;
use engine::Engine;
use evaluate::{evaluate, Progress};
use helper::Helper;
use scheme::Scheme;
use sha2::{Digest, Sha256};
use shared_types::{EvalJob, EvalResult};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

/// Takes games from hivegame.com's eval queue, searches them with the configured engine and
/// posts back which moves were inaccuracies, mistakes and blunders.
#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env = "HIVE_URL", default_value = "http://localhost:3000")]
    base_url: String,
    #[arg(long, env = "EVAL_WORKER_TOKEN", hide_env_values = true)]
    token: String,
    /// Name this worker reports under; must be unique among running workers.
    #[arg(long, env = "EVAL_WORKER_NAME")]
    worker: Option<String>,
    /// The engines file [default: ~/.config/hive-evaluator/engines.toml].
    #[arg(long, env = "EVAL_ENGINES_FILE")]
    engines_file: Option<PathBuf>,
    /// Which engine of the file to run, instead of the file's `use`.
    #[arg(long, env = "EVAL_ENGINE")]
    engine: Option<String>,
    /// Also take games the site picks itself when no user is waiting; `EVAL_AUTO=false` keeps
    /// the worker to user requests, e.g. for a gentle first start.
    #[arg(long, env = "EVAL_AUTO", default_value_t = true, action = clap::ArgAction::Set)]
    auto: bool,
}

const IDLE_POLL: Duration = Duration::from_secs(5);
const ERROR_BACKOFF: Duration = Duration::from_secs(30);
const PROGRESS_EVERY: Duration = Duration::from_secs(5);
/// A finished eval is minutes of work: delivering it is retried through a deploy or a network
/// blip, for about as long as the site keeps the eval this worker's without a heartbeat.
const DELIVER_RETRY_FIRST: Duration = Duration::from_secs(5);
const DELIVER_RETRY_MOST: Duration = Duration::from_secs(60);
const DELIVER_FOR: Duration = Duration::from_secs(240);

struct Workers {
    _helpers: Vec<Helper>,
    engines: Vec<Engine>,
}

impl Workers {
    async fn start(config: &EngineConfig) -> Result<Self> {
        let mut helpers = Vec::new();
        let mut engines = Vec::new();
        match &config.helper {
            Some(helper) => {
                for i in 0..helper.count {
                    let helper = Helper::start(helper, helper.first_port + i).await?;
                    for _ in 0..config.count {
                        engines.push(Engine::start(config, Some(helper.port)).await?);
                    }
                    helpers.push(helper);
                }
            }
            None => {
                for _ in 0..config.count {
                    engines.push(Engine::start(config, None).await?);
                }
            }
        }
        tracing::info!(
            helpers = helpers.len(),
            engines = engines.len(),
            "workers ready"
        );
        Ok(Workers {
            _helpers: helpers,
            engines,
        })
    }
}

/// Short digest of the files that tell one version of the engine from another; None when the
/// config names none.
fn fingerprint(files: &[PathBuf]) -> Result<Option<String>> {
    if files.is_empty() {
        return Ok(None);
    }
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(std::fs::read(file).with_context(|| format!("reading {}", file.display()))?);
    }
    let digest = hasher.finalize();
    Ok(Some(
        digest.iter().take(6).map(|b| format!("{b:02x}")).collect(),
    ))
}

fn default_engines_file() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set; pass --engines-file")?;
    Ok(PathBuf::from(home).join(".config/hive-evaluator/engines.toml"))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Dropping `run` on a signal drops the helpers and engines, which kills them; a plain
    // SIGTERM would otherwise leave the helpers running and holding their ports.
    tokio::select! {
        result = run() => result,
        signal = shutdown_signal() => {
            signal?;
            tracing::info!("shutting down");
            Ok(())
        }
    }
}

async fn shutdown_signal() -> Result<()> {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("installing the SIGTERM handler")?;
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    Ok(())
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
    let engines_file = match &args.engines_file {
        Some(file) => file.clone(),
        None => default_engines_file()?,
    };
    let (engine_name, config) = EngineConfig::load(&engines_file, args.engine.as_deref())?;
    let scheme = config.scheme();
    let mut api = Api::new(&args.base_url, &args.token, &worker)?;
    api.auto = args.auto;
    let digest = fingerprint(&config.fingerprint)?;

    let mut workers = Workers::start(&config).await?;
    let engine_id = [
        workers.engines.first().map(|e| e.id.clone()),
        digest,
        Some(scheme.label()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    tracing::info!(%worker, engine = %engine_name, %engine_id, auto = args.auto, "waiting for evals");

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
            tracing::warn!("restarting the helpers and engines");
            drop(std::mem::replace(
                &mut workers,
                Workers {
                    _helpers: Vec::new(),
                    engines: Vec::new(),
                },
            ));
            workers = loop {
                match Workers::start(&config).await {
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

    let lost = progress.cancelled.load(Ordering::Relaxed);
    let healthy = match outcome {
        Ok(moves) if !lost => {
            // The heartbeat goes on while the result is delivered, so the site does not hand
            // the eval to another worker in the meantime.
            match deliver(api, eval_id, engine_id, &EvalResult { moves }).await {
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
            reporter.abort();
            tracing::error!(%eval_id, "eval failed: {e:#}");
            if let Err(report) = api.fail(eval_id, &format!("{e:#}")).await {
                tracing::error!(%eval_id, "reporting the failure failed: {report:#}");
            }
            false
        }
        Ok(_) => true,
    };
    reporter.abort();
    healthy
}

async fn deliver(
    api: &Api,
    eval_id: uuid::Uuid,
    engine_id: &str,
    result: &EvalResult,
) -> Result<Ownership> {
    let started = std::time::Instant::now();
    let mut wait = DELIVER_RETRY_FIRST;
    loop {
        match api.submit(eval_id, engine_id, result).await {
            Err(e) if worth_retrying(&e) && started.elapsed() + wait < DELIVER_FOR => {
                tracing::warn!(%eval_id, "submitting failed, trying again in {wait:?}: {e:#}");
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(DELIVER_RETRY_MOST);
            }
            delivered => return delivered,
        }
    }
}
