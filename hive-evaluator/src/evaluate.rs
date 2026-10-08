use crate::{
    engine::Engine,
    scheme::{assemble, positions_to_recheck, Scheme, Searched},
};
use anyhow::{anyhow, Result};
use shared_types::MoveEval;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
        Mutex,
    },
};

/// Share of the reported progress that the screen pass stands for. The screen searches every
/// position at a sixteenth of the sims, but a full-sims recheck covers only part of them.
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

/// Searches `plies` at `sims`, spread over all engines. The engines come back even when the
/// pass fails, so the caller decides whether to restart them.
async fn run_pass(
    engines: Vec<Engine>,
    game_type: &str,
    moves: &Arc<Vec<String>>,
    plies: Vec<usize>,
    sims: u32,
    progress: &Arc<Progress>,
) -> (Vec<Engine>, Result<HashMap<usize, Searched>>) {
    let queue = Arc::new(Mutex::new(VecDeque::from(plies)));
    let mut tasks = Vec::new();
    for mut engine in engines {
        let (queue, moves, progress) = (queue.clone(), moves.clone(), progress.clone());
        let game_type = game_type.to_string();
        tasks.push(tokio::spawn(async move {
            let mut found = Vec::new();
            loop {
                if progress.cancelled.load(Ordering::Relaxed)
                    || progress.aborted.load(Ordering::Relaxed)
                {
                    return (engine, Err(anyhow!("stopped")));
                }
                let Some(ply) = queue.lock().unwrap().pop_front() else {
                    return (engine, Ok(found));
                };
                match engine.search(&game_type, &moves, ply, sims).await {
                    Ok(searched) => {
                        found.push((ply, searched));
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
    (engines, error.map_or(Ok(results), Err))
}

pub async fn evaluate(
    engines: Vec<Engine>,
    game_type: &str,
    moves: Vec<String>,
    scheme: &Scheme,
    progress: &Arc<Progress>,
) -> (Vec<Engine>, Result<Vec<Option<MoveEval>>>) {
    let moves = Arc::new(moves);
    let positions = moves.len() + 1;

    progress.start_pass(positions, false);
    let (engines, screen) = run_pass(
        engines,
        game_type,
        &moves,
        (0..positions).collect(),
        scheme.screen,
        progress,
    )
    .await;
    let screen: Vec<Searched> = match screen {
        Ok(mut found) => (0..positions).map(|p| found.remove(&p).flatten()).collect(),
        Err(e) => return (engines, Err(e)),
    };

    let recheck = positions_to_recheck(&moves, &screen, scheme.threshold);
    progress.start_pass(recheck.len(), true);
    let (engines, full) =
        run_pass(engines, game_type, &moves, recheck, scheme.full, progress).await;
    let full: Vec<Option<Searched>> = match full {
        Ok(mut found) => (0..positions).map(|p| found.remove(&p)).collect(),
        Err(e) => return (engines, Err(e)),
    };

    let result = assemble(&moves, &screen, &full, scheme);
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
