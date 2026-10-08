use crate::scheme::{PositionEval, Searched};
use anyhow::{anyhow, bail, Context, Result};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

/// The flags StockBee's own bot plays with, minus pondering and the opening book.
const SEARCH_FLAGS: &[&str] = &[
    "--tf-fallback=none",
    "--tf-retry-ms=2000",
    "--tf-open-plies=0",
    "--tf-solver",
    "--tf-sat-select",
    "--tf-race",
    "--tf-tt",
    "--threads=1",
    "--tt=256",
];
const COMMAND_TIMEOUT: Duration = Duration::from_secs(600);

/// A StockBee process spoken to over UHP.
pub struct Engine {
    _child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    pub id: String,
}

impl Engine {
    pub async fn start(binary: &Path, eval_server_port: u16) -> Result<Self> {
        let mut child = Command::new(binary)
            .arg(format!("--transformer-port={eval_server_port}"))
            .args(SEARCH_FLAGS)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("starting {}", binary.display()))?;
        let stdin = child.stdin.take().context("engine stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("engine stdout")?).lines();
        let mut engine = Engine {
            _child: child,
            stdin,
            stdout,
            id: String::new(),
        };
        // UHP engines greet with their id before any command.
        let banner = engine.read_reply().await?;
        engine.id = banner
            .iter()
            .find_map(|l| l.strip_prefix("id name ").or_else(|| l.strip_prefix("id ")))
            .unwrap_or("unknown")
            .to_string();
        Ok(engine)
    }

    async fn read_reply(&mut self) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        loop {
            let line = timeout(COMMAND_TIMEOUT, self.stdout.next_line())
                .await
                .context("engine timed out")??
                .ok_or_else(|| anyhow!("engine exited"))?;
            if line == "ok" {
                return Ok(lines);
            }
            lines.push(line);
        }
    }

    async fn command(&mut self, command: &str) -> Result<Vec<String>> {
        self.stdin
            .write_all(format!("{command}\n").as_bytes())
            .await?;
        self.stdin.flush().await?;
        self.read_reply().await
    }

    /// Searches the position after the first `ply` moves from a fresh tree, so the result
    /// depends only on the position and not on what this engine searched before.
    pub async fn search(
        &mut self,
        game_type: &str,
        moves: &[String],
        ply: usize,
        sims: u32,
    ) -> Result<Searched> {
        let mut state = self.command(&format!("newgame {game_type}")).await?;
        for mv in &moves[..ply] {
            state = self.command(&format!("play {mv}")).await?;
            if let Some(err) = state.iter().find(|l| l.starts_with("err")) {
                bail!("replaying {mv} at ply {ply}: {err}");
            }
        }
        if game_over(&state) || self.command("validmoves").await? == ["pass"] {
            return Ok(None);
        }
        let reply = self.command(&format!("analyze sims {sims} topk 1")).await?;
        let mut eval = parse_analyze(&reply)?;
        if let Some(played) = moves.get(ply) {
            if eval.best != *played && self.same_move(&eval.best, played).await? {
                // UHP names a destination relative to any neighbour, so the engine can write the
                // played move differently ("wG2 -wA1" for "wG2 wB1\"). Keep the game's spelling,
                // or the player is graded for the very move the engine recommends.
                eval.best = played.clone();
                if let Some(first) = eval.line.first_mut() {
                    *first = played.clone();
                }
            }
        }
        Ok(Some(eval))
    }

    /// Whether two moves from the current position lead to the same position.
    async fn same_move(&mut self, a: &str, b: &str) -> Result<bool> {
        let mut hashes = Vec::new();
        for mv in [a, b] {
            let played = self.command(&format!("play {mv}")).await?;
            if played.iter().any(|l| l.starts_with("err")) {
                return Ok(false);
            }
            hashes.push(self.command("hash").await?);
            self.command("undo").await?;
        }
        Ok(hashes[0] == hashes[1])
    }
}

/// True when the game string in a `play`/`newgame` reply shows a finished game.
fn game_over(reply: &[String]) -> bool {
    reply.iter().any(|line| {
        matches!(
            line.split(';').nth(1),
            Some("WhiteWins" | "BlackWins" | "Draw")
        )
    })
}

fn parse_analyze(reply: &[String]) -> Result<PositionEval> {
    if let Some(err) = reply.iter().find(|l| l.starts_with("err")) {
        bail!("analyze failed: {err}");
    }
    let cand = reply
        .iter()
        .find_map(|l| l.strip_prefix("cand "))
        .ok_or_else(|| anyhow!("analyze gave no candidate: {reply:?}"))?;
    // Fields are " | "-separated because UHP moves contain spaces.
    let mut fields = cand.split(" | ");
    let best = fields.next().unwrap_or_default().to_string();
    let mut winprob = None;
    let mut line = Vec::new();
    for field in fields {
        if let Some(v) = field.strip_prefix("winprob ") {
            winprob = v.trim().parse::<f32>().ok();
        } else if let Some(v) = field.strip_prefix("pv ") {
            line = v.split(';').map(|m| m.trim().to_string()).collect();
        }
    }
    let winprob = winprob.ok_or_else(|| anyhow!("no winprob in {cand}"))?;
    if best.is_empty() || !(0.0..=100.0).contains(&winprob) {
        bail!("malformed candidate: {cand}");
    }
    Ok(PositionEval {
        winprob,
        best,
        line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    #[test]
    fn parses_stockbees_analyze_reply() {
        let reply = lines(
            "info analyze sims 800 leaves 553 depth 8\n\
             cand bP wL/ | prior 0.151676 | visits 123 | winprob 37.166 | pv bP wL/;wA1 wL\\;bM bP/",
        );
        assert_eq!(
            parse_analyze(&reply).unwrap(),
            PositionEval {
                winprob: 37.166,
                best: "bP wL/".to_string(),
                line: vec!["bP wL/".into(), "wA1 wL\\".into(), "bM bP/".into()],
            }
        );
    }

    #[test]
    fn an_analyze_error_is_an_error_not_an_empty_position() {
        let reply = lines("err transformer unavailable or no legal moves");
        assert!(parse_analyze(&reply).is_err());
    }

    /// Needs a built StockBee; skipped when there is none. Searching is not needed, so no eval
    /// server either.
    #[tokio::test]
    async fn two_spellings_of_one_move_are_the_same_move() {
        let binary =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../stockbee/build/stockbee");
        if !binary.exists() {
            eprintln!("skipped: no StockBee build at {}", binary.display());
            return;
        }
        let mut engine = Engine::start(&binary, 1).await.unwrap();
        engine.command("newgame Base+MLP").await.unwrap();
        // The local game that exposed this: its 17th move, written two ways.
        for mv in "wL;bL wL-;wP /wL;bQ bL/;wQ \\wP;bM bQ\\;wA1 \\wL;bP bM/;wA1 \\bP;bA1 bL\\;\
                   wG1 \\wA1;bA2 bM\\;wB1 -wG1;bA2 -wB1;wG2 wG1/;bA3 bM\\"
            .split(';')
        {
            engine.command(&format!("play {mv}")).await.unwrap();
        }
        assert!(engine.same_move("wG2 wB1\\", "wG2 -wA1").await.unwrap());
        assert!(!engine.same_move("wG2 wB1\\", "wB2 -wA1").await.unwrap());
    }

    #[test]
    fn a_finished_game_string_is_game_over() {
        assert!(game_over(&lines("Base+PLM;WhiteWins;Black[20];wL;bL wL\\")));
        assert!(!game_over(&lines(
            "Base+PLM;InProgress;White[20];wL;bL wL\\"
        )));
        assert!(!game_over(&lines("Base+PLM;NotStarted;White[1]")));
    }
}
