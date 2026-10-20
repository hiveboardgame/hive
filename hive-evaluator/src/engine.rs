use crate::{
    config::{with_port, EngineConfig},
    rules::same_move,
    scheme::{PositionEval, Searched},
};
use anyhow::{anyhow, bail, Context, Result};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(600);

/// An engine process spoken to over UHP; see the README, "Engines".
pub struct Engine {
    _child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    analyze: String,
    pub id: String,
}

impl Engine {
    /// Starts one engine; `helper_port` fills in `{port}` in its arguments.
    pub async fn start(config: &EngineConfig, helper_port: Option<u16>) -> Result<Self> {
        let mut command = Command::new(&config.command);
        command
            .args(with_port(&config.args, helper_port))
            .envs(&config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(dir) = &config.dir {
            command.current_dir(dir);
        }
        let mut child = command
            .spawn()
            .with_context(|| format!("starting {}", config.command.display()))?;
        let stdin = child.stdin.take().context("engine stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("engine stdout")?).lines();
        let mut engine = Engine {
            _child: child,
            stdin,
            stdout,
            analyze: config.analyze.clone(),
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

    /// Searches the position after the first `ply` moves with `budget`, set up afresh so the
    /// result depends only on the position and not on what this engine searched before.
    pub async fn search(
        &mut self,
        game_type: &str,
        moves: &[String],
        ply: usize,
        budget: u32,
    ) -> Result<Searched> {
        let state = self.command(&newgame(game_type, &moves[..ply])).await?;
        if let Some(err) = state.iter().find(|l| l.starts_with("err")) {
            bail!("setting up the position after {ply} moves: {err}");
        }
        if self.command("validmoves").await? == ["pass"] {
            return Ok(Searched::OnlyPass);
        }
        let analyze = self.analyze.replace("{budget}", &budget.to_string());
        let reply = self.command(&analyze).await?;
        let mut eval = parse_analyze(&reply)?;
        if let Some(played) = moves.get(ply) {
            if eval.best != *played && same_move(game_type, &moves[..ply], &eval.best, played)? {
                // UHP names a destination relative to any neighbour, so the engine can write the
                // played move differently ("wG2 -wA1" for "wG2 wB1\"). Keep the game's spelling,
                // or the player is graded for the very move the engine recommends.
                eval.best = played.clone();
                if let Some(first) = eval.line.first_mut() {
                    *first = played.clone();
                }
            }
        }
        Ok(Searched::Position(eval))
    }
}

/// The position after `moves` as one UHP game string, so setting it up is a single round trip
/// however long the game.
fn newgame(game_type: &str, moves: &[String]) -> String {
    if moves.is_empty() {
        return format!("newgame {game_type}");
    }
    let color = if moves.len() % 2 == 0 {
        "White"
    } else {
        "Black"
    };
    format!(
        "newgame {game_type};InProgress;{color}[{}];{}",
        moves.len() / 2 + 1,
        moves.join(";")
    )
}

/// Reads `cand <move> | winprob <0-100> | pv <move>;<move>…` from the analysis reply; other
/// fields and lines are ignored.
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
    fn parses_an_analyze_reply() {
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
        let reply = lines("err no net loaded");
        assert!(parse_analyze(&reply).is_err());
    }

    #[test]
    fn a_position_is_set_up_with_one_game_string_naming_the_side_to_move() {
        assert_eq!(newgame("Base+MLP", &[]), "newgame Base+MLP");
        let moves = [
            "wL".to_string(),
            r"bL wL\".to_string(),
            "wQ /wL".to_string(),
        ];
        assert_eq!(
            newgame("Base+MLP", &moves[..1]),
            "newgame Base+MLP;InProgress;Black[1];wL"
        );
        assert_eq!(
            newgame("Base+MLP", &moves),
            r"newgame Base+MLP;InProgress;Black[2];wL;bL wL\;wQ /wL"
        );
    }

    /// A UHP engine in a few lines of shell: it knows the positions after three and four moves,
    /// can only pass after three, and analyses at budget 7 only.
    fn fake_engine() -> EngineConfig {
        let script = r#"
            say() { printf '%s\n' "$@" ok; }
            say "id Fake 1.0"
            while read -r line; do
                case "$line" in
                    "newgame Base+MLP;InProgress;White[3];"*) plies=4; say "${line#newgame }" ;;
                    "newgame Base+MLP;InProgress;Black[2];"*) plies=3; say "${line#newgame }" ;;
                    newgame*) say "err not this position" ;;
                    validmoves) if [ "$plies" = 3 ]; then say pass; else say "wA1 bL-;wQ -wL"; fi ;;
                    "think 7") say 'cand wA1 bL- | winprob 61.5 | pv wA1 bL-;bQ bL\' ;;
                    *) say "err unknown command" ;;
                esac
            done
        "#;
        let (_, mut config) = EngineConfig::pick(
            r#"
            [engines.fake]
            command = "sh"
            analyze = "think {budget}"
            screen = 7
            full = 7
            "#,
            None,
        )
        .unwrap();
        config.args = vec!["-c".into(), script.into()];
        config
    }

    #[tokio::test]
    async fn any_engine_that_speaks_the_protocol_searches_positions() {
        let moves: Vec<String> = ["wL", "bL wL-", "wQ /wL", "bQ bL/", "wA1 bL-"]
            .map(String::from)
            .to_vec();
        let mut engine = Engine::start(&fake_engine(), None).await.unwrap();
        assert_eq!(engine.id, "Fake 1.0");

        assert_eq!(
            engine.search("Base+MLP", &moves, 4, 7).await.unwrap(),
            Searched::Position(PositionEval {
                winprob: 61.5,
                best: "wA1 bL-".into(),
                line: vec!["wA1 bL-".into(), r"bQ bL\".into()],
            })
        );
        assert_eq!(
            engine.search("Base+MLP", &moves, 3, 7).await.unwrap(),
            Searched::OnlyPass
        );
        assert!(engine.search("Base+MLP", &moves, 2, 7).await.is_err());
        assert!(
            engine.search("Base+MLP", &moves, 4, 8).await.is_err(),
            "the budget reaches the engine"
        );
    }
}
