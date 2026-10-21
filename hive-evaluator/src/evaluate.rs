use crate::{
    engine::Engine,
    rules::ending,
    scheme::{assemble, positions_to_recheck, Scheme, Searched},
};
use anyhow::{anyhow, Result};
use shared_types::{MoveEval, MAX_EVAL_LINE};
use std::{
    collections::HashMap,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

/// Share of the reported progress that the screen pass stands for. The screen searches every
/// position with a fraction of the budget, but a full recheck covers only part of them.
const SCREEN_SHARE: f64 = 0.2;

#[derive(Default)]
pub struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
    in_full_pass: AtomicBool,
    /// Set when the server took the eval away.
    pub cancelled: AtomicBool,
    /// Set when an engine failed and the pass cannot complete.
    aborted: AtomicBool,
}

impl Progress {
    pub fn percent(&self) -> u8 {
        let done = self.done.load(Ordering::Relaxed) as f64;
        let total = self.total.load(Ordering::Relaxed).max(1) as f64;
        let pass = (done / total).min(1.0);
        let pct = if self.in_full_pass.load(Ordering::Relaxed) {
            SCREEN_SHARE + (1.0 - SCREEN_SHARE) * pass
        } else {
            SCREEN_SHARE * pass
        };
        (pct * 100.0).floor().min(99.0) as u8
    }

    fn start_pass(&self, total: usize, full: bool) {
        self.done.store(0, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        self.in_full_pass.store(full, Ordering::Relaxed);
    }
}

/// Does `work` for every ply in `jobs`, spread over all engines. The engines come back even
/// when a job fails, so the caller decides whether to restart them.
async fn spread<J, R, W, F>(
    engines: Vec<Engine>,
    jobs: Vec<(usize, J)>,
    progress: &Arc<Progress>,
    work: W,
) -> (Vec<Engine>, Result<HashMap<usize, R>>)
where
    J: Clone + Send + Sync + 'static,
    R: Send + 'static,
    W: Fn(Engine, usize, J) -> F + Clone + Send + 'static,
    F: Future<Output = (Engine, Result<R>)> + Send + 'static,
{
    let jobs = Arc::new(jobs);
    let next = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for mut engine in engines {
        let (jobs, next, progress, work) =
            (jobs.clone(), next.clone(), progress.clone(), work.clone());
        tasks.push(tokio::spawn(async move {
            let mut found = Vec::new();
            loop {
                // What stopped the pass is reported once, by whoever hit it, not as a "stopped"
                // from each engine that merely gave up.
                if progress.cancelled.load(Ordering::Relaxed)
                    || progress.aborted.load(Ordering::Relaxed)
                {
                    return (engine, Ok(found));
                }
                let Some((ply, job)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)).cloned()
                else {
                    return (engine, Ok(found));
                };
                let (back, done) = work(engine, ply, job).await;
                engine = back;
                match done {
                    Ok(done) => {
                        found.push((ply, done));
                        progress.done.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        progress.aborted.store(true, Ordering::Relaxed);
                        return (engine, Err(e));
                    }
                }
            }
        }));
    }
    let mut engines = Vec::new();
    let mut results = HashMap::new();
    let mut error = None;
    for task in tasks {
        match task.await {
            Ok((engine, outcome)) => {
                engines.push(engine);
                match outcome {
                    Ok(found) => results.extend(found),
                    Err(e) => error = error.or(Some(e)),
                }
            }
            Err(e) => error = error.or(Some(anyhow!("search task panicked: {e}"))),
        }
    }
    if error.is_none() && progress.cancelled.load(Ordering::Relaxed) {
        error = Some(anyhow!("the site took the eval back"));
    }
    (engines, error.map_or(Ok(results), Err))
}

/// Searches `plies` with `budget`; the final position is scored by `ending` instead when the
/// game ended on the board.
async fn run_pass(
    engines: Vec<Engine>,
    game_type: &str,
    moves: &Arc<Vec<String>>,
    ending: Option<f32>,
    plies: Vec<usize>,
    budget: u32,
    progress: &Arc<Progress>,
) -> (Vec<Engine>, Result<HashMap<usize, Searched>>) {
    let (game_type, moves) = (Arc::new(game_type.to_string()), moves.clone());
    let jobs = plies.into_iter().map(|ply| (ply, ())).collect();
    spread(engines, jobs, progress, move |mut engine, ply, ()| {
        let (game_type, moves) = (game_type.clone(), moves.clone());
        async move {
            let searched = match ending.filter(|_| ply == moves.len()) {
                Some(winprob) => Ok(Searched::GameOver { winprob }),
                None => engine.search(&game_type, &moves, ply, budget).await,
            };
            (engine, searched)
        }
    })
    .await
}

/// The engine's line instead of each marked move, from the position before it.
async fn find_lines(
    engines: Vec<Engine>,
    game_type: &str,
    moves: &Arc<Vec<String>>,
    marked: Vec<(usize, String)>,
    budget: u32,
    progress: &Arc<Progress>,
) -> (Vec<Engine>, Result<HashMap<usize, Vec<String>>>) {
    let (game_type, moves) = (Arc::new(game_type.to_string()), moves.clone());
    spread(engines, marked, progress, move |mut engine, ply, best| {
        let (game_type, moves) = (game_type.clone(), moves.clone());
        async move {
            let line = engine
                .line(&game_type, &moves[..ply], &best, budget, MAX_EVAL_LINE)
                .await;
            (engine, line)
        }
    })
    .await
}

pub async fn evaluate(
    engines: Vec<Engine>,
    game_type: &str,
    moves: Vec<String>,
    scheme: &Scheme,
    progress: &Arc<Progress>,
) -> (Vec<Engine>, Result<Vec<Option<MoveEval>>>) {
    let ending = match ending(game_type, &moves) {
        Ok(ending) => ending,
        Err(e) => return (engines, Err(e)),
    };
    let moves = Arc::new(moves);
    let positions = moves.len() + 1;

    progress.start_pass(positions, false);
    let (engines, screen) = run_pass(
        engines,
        game_type,
        &moves,
        ending,
        (0..positions).collect(),
        scheme.screen,
        progress,
    )
    .await;
    let screen: Vec<Searched> = match screen.and_then(|mut found| {
        (0..positions)
            .map(|p| {
                found
                    .remove(&p)
                    .ok_or_else(|| anyhow!("position {p} was not searched"))
            })
            .collect()
    }) {
        Ok(screen) => screen,
        Err(e) => return (engines, Err(e)),
    };

    let mut engines = engines;
    let mut full: Vec<Option<Searched>> = vec![None; positions];
    progress.start_pass(0, true);
    loop {
        let recheck = positions_to_recheck(&moves, &screen, &full, scheme.threshold);
        if recheck.is_empty() {
            break;
        }
        progress.total.fetch_add(recheck.len(), Ordering::Relaxed);
        let (back, found) = run_pass(
            engines,
            game_type,
            &moves,
            ending,
            recheck,
            scheme.full,
            progress,
        )
        .await;
        engines = back;
        match found {
            Ok(found) => {
                for (p, searched) in found {
                    full[p] = Some(searched);
                }
            }
            Err(e) => return (engines, Err(e)),
        }
    }

    let mut result = assemble(&moves, &screen, &full, scheme);
    // Only a marked move's line is shown, and a line of the engine's own best moves holds up
    // where the tail of one search does not.
    let marked: Vec<(usize, String)> = result
        .iter()
        .enumerate()
        .filter_map(|(ply, eval)| {
            let eval = eval.as_ref()?;
            eval.grade().map(|_| (ply, eval.best.clone()))
        })
        .collect();
    progress.total.fetch_add(marked.len(), Ordering::Relaxed);
    let (engines, lines) =
        find_lines(engines, game_type, &moves, marked, scheme.line, progress).await;
    match lines {
        Ok(lines) => {
            for (ply, line) in lines {
                if let Some(eval) = result[ply].as_mut() {
                    eval.line = line;
                }
            }
        }
        Err(e) => return (engines, Err(e)),
    }
    (engines, Ok(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_gives_the_screen_a_fifth_and_never_claims_done() {
        let p = Progress::default();
        p.start_pass(10, false);
        p.done.store(10, Ordering::Relaxed);
        assert_eq!(p.percent(), 20);
        p.start_pass(4, true);
        p.done.store(2, Ordering::Relaxed);
        assert_eq!(p.percent(), 60);
        p.done.store(4, Ordering::Relaxed);
        assert_eq!(p.percent(), 99);
    }

    #[test]
    fn a_full_pass_with_nothing_to_recheck_is_not_a_division_by_zero() {
        let p = Progress::default();
        p.start_pass(0, true);
        assert_eq!(p.percent(), 20);
    }
}
