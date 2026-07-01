use std::{
    io::{self, BufRead, Write},
    time::Duration,
};

use hudsoni::{History, State};

use crate::{
    game::{Action, Game},
    search::Limits,
};

const NAME: &str = "philanthus";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const CAPABILITIES: &str = "Mosquito;Ladybug;Pillbug";

pub fn run_uhp() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut engine = UhpEngine::new();

    write_info(&mut out)?;
    writeln!(out, "ok")?;
    out.flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "exit" {
            break;
        }
        engine.handle(trimmed, &mut out)?;
    }
    Ok(())
}

fn write_info<W: Write>(out: &mut W) -> io::Result<()> {
    writeln!(out, "id {NAME} {VERSION}")?;
    writeln!(out, "{CAPABILITIES}")
}

struct UhpEngine {
    state: Option<State>,
}

impl UhpEngine {
    fn new() -> Self {
        Self { state: None }
    }

    fn handle<W: Write>(&mut self, line: &str, out: &mut W) -> io::Result<()> {
        if let Err(message) = self.dispatch(line, out) {
            writeln!(out, "err {message}")?;
        }
        writeln!(out, "ok")?;
        out.flush()
    }

    fn dispatch<W: Write>(&mut self, line: &str, out: &mut W) -> Result<(), String> {
        match line.split_whitespace().next().unwrap_or_default() {
            "info" => write_info(out).map_err(stringify),
            "newgame" => self.cmd_newgame(line, out),
            "validmoves" => self.cmd_validmoves(out),
            "bestmove" => self.cmd_bestmove(line, out),
            "play" => self.cmd_play(line, out),
            "pass" => self.cmd_pass(out),
            "undo" => self.cmd_undo(line, out),
            "options" => Ok(()),
            "perft" => self.cmd_perft(line, out),
            other => Err(format!("invalid command: {other}")),
        }
    }

    fn cmd_newgame<W: Write>(&mut self, line: &str, out: &mut W) -> Result<(), String> {
        let rest = line
            .strip_prefix("newgame")
            .map(str::trim)
            .unwrap_or_default();
        let history = History::from_uhp_str(rest.to_string()).map_err(stringify)?;
        let state = State::new_from_history(&history).map_err(stringify)?;
        self.state = Some(state);
        self.write_game_string(out)
    }

    fn cmd_validmoves<W: Write>(&self, out: &mut W) -> Result<(), String> {
        let state = self.state()?;
        let game = Game::from_state(state);
        let mut strings = Vec::new();
        for action in game.legal_actions() {
            strings.push(action_to_movestring(state, &action)?);
        }
        writeln!(out, "{}", strings.join(";")).map_err(stringify)
    }

    fn cmd_bestmove<W: Write>(&self, line: &str, out: &mut W) -> Result<(), String> {
        let state = self.state()?;
        let mut game = Game::from_state(state);
        let outcome = crate::search::search_outcome(&mut game, parse_limits(line))
            .ok_or("no legal moves available")?;
        eprintln!("depth {} score {}", outcome.completed_depth, outcome.score);
        let movestring = action_to_movestring(state, &outcome.action)?;
        writeln!(out, "{movestring}").map_err(stringify)
    }

    fn cmd_play<W: Write>(&mut self, line: &str, out: &mut W) -> Result<(), String> {
        let rest = line.strip_prefix("play").map(str::trim).unwrap_or_default();
        let mut tokens = rest.split_whitespace();
        let piece = tokens.next().ok_or("play requires a move")?;
        let position = tokens.next().unwrap_or_default();
        self.state_mut()?
            .play_turn_from_history(piece, position)
            .map_err(stringify)?;
        self.write_game_string(out)
    }

    fn cmd_pass<W: Write>(&mut self, out: &mut W) -> Result<(), String> {
        self.state_mut()?
            .play_turn_from_history("pass", "")
            .map_err(stringify)?;
        self.write_game_string(out)
    }

    fn cmd_undo<W: Write>(&mut self, line: &str, out: &mut W) -> Result<(), String> {
        let rest = line.strip_prefix("undo").map(str::trim).unwrap_or_default();
        let count: usize = if rest.is_empty() {
            1
        } else {
            rest.parse()
                .map_err(|_| format!("invalid undo count: {rest}"))?
        };
        {
            let state = self.state_mut()?;
            for _ in 0..count {
                state.undo();
            }
        }
        self.write_game_string(out)
    }

    fn cmd_perft<W: Write>(&self, line: &str, out: &mut W) -> Result<(), String> {
        let rest = line
            .strip_prefix("perft")
            .map(str::trim)
            .unwrap_or_default();
        let depth: usize = rest
            .parse()
            .map_err(|_| format!("invalid perft depth: {rest}"))?;
        let state = self.state()?;
        let mut game = Game::from_state(state);
        writeln!(out, "{}", game.perft(depth)).map_err(stringify)
    }

    fn write_game_string<W: Write>(&self, out: &mut W) -> Result<(), String> {
        writeln!(out, "{}", self.state()?.to_uhp_game_string()).map_err(stringify)
    }

    fn state(&self) -> Result<&State, String> {
        self.state
            .as_ref()
            .ok_or_else(|| "no game in progress".to_string())
    }

    fn state_mut(&mut self) -> Result<&mut State, String> {
        self.state
            .as_mut()
            .ok_or_else(|| "no game in progress".to_string())
    }
}

fn stringify<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

const DEFAULT_TIME: Duration = Duration::from_secs(1);

fn parse_limits(line: &str) -> Limits {
    let mut tokens = line.split_whitespace().skip(1);
    match tokens.next() {
        Some("depth") => {
            if let Some(depth) = tokens.next().and_then(|value| value.parse::<u32>().ok()) {
                return Limits::depth(depth);
            }
        }
        Some("time") => {
            if let Some(duration) = tokens.next().and_then(parse_hms) {
                return Limits::time(duration);
            }
        }
        _ => {}
    }
    Limits::time(DEFAULT_TIME)
}

fn parse_hms(value: &str) -> Option<Duration> {
    let mut parts = value.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(Duration::from_secs(hours * 3600 + minutes * 60) + Duration::from_secs_f64(seconds))
}

fn action_to_movestring(state: &State, action: &Action) -> Result<String, String> {
    if let Action::Pass = action {
        return Ok("pass".to_string());
    }
    let mut probe = state.clone();
    match action {
        Action::Move(piece, _, to) | Action::Place(piece, to) => {
            probe
                .play_turn_from_position(*piece, *to)
                .map_err(stringify)?;
        }
        Action::Pass => unreachable!(),
    }
    probe
        .last_move_uhp()
        .ok_or_else(|| "engine recorded no move".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(engine: &mut UhpEngine, line: &str) -> String {
        let mut buf = Vec::new();
        engine.handle(line, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn plays_a_legal_game_over_uhp() {
        let mut engine = UhpEngine::new();

        let out = run(&mut engine, "newgame Base+MLP");
        assert!(out.starts_with("Base+MLP;NotStarted;White[1]"), "{out}");
        assert!(out.trim_end().ends_with("ok"));

        let out = run(&mut engine, "validmoves");
        assert!(!out.lines().next().unwrap().is_empty(), "{out}");

        let out = run(&mut engine, "bestmove time 00:00:01");
        let mv = out.lines().next().unwrap().to_string();
        assert!(!mv.is_empty() && mv != "ok", "{out}");

        let out = run(&mut engine, &format!("play {mv}"));
        assert!(out.contains("InProgress"), "{out}");

        let out = run(&mut engine, "undo");
        assert!(out.contains("NotStarted"), "{out}");
    }

    #[test]
    fn uhp_perft_matches_game_perft() {
        let mut engine = UhpEngine::new();
        run(&mut engine, "newgame Base+MLP");
        let out = run(&mut engine, "perft 3");
        let nodes: u64 = out.lines().next().unwrap().parse().unwrap();
        let mut game = Game::from_state(engine.state.as_ref().unwrap());
        assert_eq!(nodes, game.perft(3));
    }
}
