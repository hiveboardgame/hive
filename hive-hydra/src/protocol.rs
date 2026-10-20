use serde::{de::Error as _, Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Debug, Serialize)]
pub enum ClientRequest {
    Auth(String),
    Resync,
    Game { game_id: String, action: GameAction },
    Challenge(ChallengeAction),
}

#[derive(Debug, Serialize)]
pub enum GameAction {
    PlayAt { turn: usize, notation: String },
}

#[derive(Debug, Serialize)]
pub enum ChallengeAction {
    Accept(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub uid: String,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Game {
    pub game_id: String,
    pub current_player_id: String,
    pub finished: bool,
    pub game_type: String,
    pub history: Vec<(String, String)>,
    pub turn: usize,
    pub game_start: String,
    // A finished game sends `{"Finished": <result>}` here, not a string.
    pub game_status: Value,
}

#[derive(Debug, Deserialize)]
pub struct Challenge {
    pub challenge_id: String,
    pub opponent: Option<User>,
}

#[derive(Debug, Deserialize)]
pub struct LobbySnapshot {
    pub urgent_games: Vec<Game>,
    pub challenges: Vec<Challenge>,
}

#[derive(Debug)]
pub enum Incoming {
    Authenticated(User),
    Games(Vec<Game>),
    Challenges(Vec<Challenge>),
    LobbySnapshot(LobbySnapshot),
    ResyncRetry { retry_after_ms: u64 },
    Error(String),
    Refused(Value),
    Other(String),
}

#[derive(Deserialize)]
enum Envelope {
    Ok(Value),
    Err(Value),
}

#[derive(Deserialize)]
struct ResyncRetry {
    retry_after_ms: u64,
}

#[derive(Deserialize)]
struct Reaction {
    game: Game,
}

pub fn parse(text: &str) -> Result<Incoming, serde_json::Error> {
    match serde_json::from_str(text)? {
        Envelope::Err(refusal) => Ok(Incoming::Refused(refusal)),
        Envelope::Ok(message) => parse_message(message),
    }
}

fn parse_message(message: Value) -> Result<Incoming, serde_json::Error> {
    let (tag, payload) = split_tag(message)?;
    Ok(match tag.as_str() {
        "Authenticated" => Incoming::Authenticated(serde_json::from_value(payload)?),
        "Game" => parse_game_update(payload)?,
        "Challenge" => parse_challenge_update(payload)?,
        "LobbySnapshot" => Incoming::LobbySnapshot(serde_json::from_value(payload)?),
        "ResyncRetry" => {
            let ResyncRetry { retry_after_ms } = serde_json::from_value(payload)?;
            Incoming::ResyncRetry { retry_after_ms }
        }
        "Error" => Incoming::Error(serde_json::from_value(payload)?),
        _ => Incoming::Other(tag),
    })
}

fn parse_game_update(update: Value) -> Result<Incoming, serde_json::Error> {
    let (tag, payload) = split_tag(update)?;
    Ok(match tag.as_str() {
        "Reaction" => {
            let Reaction { game } = serde_json::from_value(payload)?;
            Incoming::Games(vec![game])
        }
        "Urgent" => Incoming::Games(serde_json::from_value(payload)?),
        _ => Incoming::Other(format!("Game.{tag}")),
    })
}

fn parse_challenge_update(update: Value) -> Result<Incoming, serde_json::Error> {
    let (tag, payload) = split_tag(update)?;
    Ok(match tag.as_str() {
        "Created" | "Direct" => Incoming::Challenges(vec![serde_json::from_value(payload)?]),
        _ => Incoming::Other(format!("Challenge.{tag}")),
    })
}

fn split_tag(value: Value) -> Result<(String, Value), serde_json::Error> {
    match value {
        Value::String(tag) => Ok((tag, Value::Null)),
        Value::Object(map) if map.len() == 1 => map
            .into_iter()
            .next()
            .ok_or_else(|| serde_json::Error::custom("empty variant")),
        other => Err(serde_json::Error::custom(format!(
            "expected a variant, got {other}"
        ))),
    }
}

impl Game {
    pub fn is_turn_of(&self, uid: &str) -> bool {
        !self.finished && self.current_player_id == uid && !self.awaits_ready_start()
    }

    fn awaits_ready_start(&self) -> bool {
        self.game_start == "Ready" && self.game_status == "NotStarted"
    }

    pub fn position_key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.game_id.hash(&mut hasher);
        self.history.hash(&mut hasher);
        hasher.finish()
    }

    pub fn uhp_game_string(&self) -> String {
        let game_type = if self.game_type.starts_with("Base") {
            self.game_type.clone()
        } else {
            format!("Base+{}", self.game_type)
        };
        let state = if self.history.is_empty() {
            "NotStarted"
        } else {
            "InProgress"
        };
        let played = self.history.len();
        let color = if played.is_multiple_of(2) {
            "White"
        } else {
            "Black"
        };
        let mut game_string = format!("{game_type};{state};{color}[{}]", played / 2 + 1);
        for (piece, position) in &self.history {
            game_string.push(';');
            game_string.push_str(format!("{piece} {position}").trim());
        }
        game_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(history: &[(&str, &str)]) -> Game {
        Game {
            game_id: "abc".to_string(),
            current_player_id: "me".to_string(),
            finished: false,
            game_type: "MLP".to_string(),
            history: history
                .iter()
                .map(|(piece, position)| (piece.to_string(), position.to_string()))
                .collect(),
            turn: history.len(),
            game_start: "Moves".to_string(),
            game_status: json!("InProgress"),
        }
    }

    fn frame(request: &ClientRequest) -> String {
        serde_json::to_string(request).unwrap()
    }

    #[test]
    fn client_frames_match_the_documented_shapes() {
        assert_eq!(
            frame(&ClientRequest::Auth("tok".to_string())),
            r#"{"Auth":"tok"}"#
        );
        assert_eq!(frame(&ClientRequest::Resync), r#""Resync""#);
        assert_eq!(
            frame(&ClientRequest::Game {
                game_id: "abc".to_string(),
                action: GameAction::PlayAt {
                    turn: 12,
                    notation: "wA1 -bQ".to_string(),
                },
            }),
            r#"{"Game":{"game_id":"abc","action":{"PlayAt":{"turn":12,"notation":"wA1 -bQ"}}}}"#
        );
        assert_eq!(
            frame(&ClientRequest::Challenge(ChallengeAction::Accept(
                "c1".to_string()
            ))),
            r#"{"Challenge":{"Accept":"c1"}}"#
        );
    }

    #[test]
    fn parses_authenticated() {
        let text = json!({"Ok": {"Authenticated": {
            "username": "bot", "uid": "u1", "patreon": false, "bot": true, "admin": false,
            "deleted": false, "ratings": {}, "takeback": "Always", "lang": null
        }}})
        .to_string();
        let Incoming::Authenticated(user) = parse(&text).unwrap() else {
            panic!("expected Authenticated");
        };
        assert_eq!(user.uid, "u1");
        assert_eq!(user.username, "bot");
    }

    #[test]
    fn parses_reaction_and_urgent_games() {
        let game = json!({
            "game_id": "abc", "current_player_id": "me", "finished": false,
            "game_type": "MLP", "history": [["wS1", ""], ["bG1", "-wS1"]],
            "turn": 2, "game_start": "Moves", "game_status": "InProgress"
        });
        let reaction = json!({"Ok": {"Game": {"Reaction": {
            "game_action": {"Turn": {"Move": ["bG1", "-wS1"]}},
            "game": game, "game_id": "abc", "user_id": "them", "username": "them"
        }}}})
        .to_string();
        let Incoming::Games(games) = parse(&reaction).unwrap() else {
            panic!("expected Games");
        };
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].history[1], ("bG1".to_string(), "-wS1".to_string()));

        let urgent = json!({"Ok": {"Game": {"Urgent": [game, game]}}}).to_string();
        let Incoming::Games(games) = parse(&urgent).unwrap() else {
            panic!("expected Games");
        };
        assert_eq!(games.len(), 2);
    }

    #[test]
    fn parses_challenges() {
        let challenge = json!({
            "challenge_id": "c1", "opponent": {"username": "bot", "uid": "me"}, "rated": true
        });
        let text = json!({"Ok": {"Challenge": {"Direct": challenge}}}).to_string();
        let Incoming::Challenges(challenges) = parse(&text).unwrap() else {
            panic!("expected Challenges");
        };
        assert_eq!(challenges[0].challenge_id, "c1");
        assert_eq!(challenges[0].opponent.as_ref().unwrap().uid, "me");

        let removed = r#"{"Ok":{"Challenge":{"Removed":"c1"}}}"#;
        assert!(
            matches!(parse(removed).unwrap(), Incoming::Other(tag) if tag == "Challenge.Removed")
        );
    }

    #[test]
    fn parses_lobby_snapshot() {
        let text = json!({"Ok": {"LobbySnapshot": {
            "tournament_invitations": [], "tournament_organizer_invitations": [],
            "schedule_notifications": [], "urgent_games": [], "tv_games": [],
            "online_users": [], "challenges": [{"challenge_id": "c1", "opponent": null}]
        }}})
        .to_string();
        let Incoming::LobbySnapshot(snapshot) = parse(&text).unwrap() else {
            panic!("expected LobbySnapshot");
        };
        assert!(snapshot.urgent_games.is_empty());
        assert!(snapshot.challenges[0].opponent.is_none());
    }

    #[test]
    fn parses_refusals_errors_and_unknown_messages() {
        let refused = r#"{"Err":{"Game":{"game_id":"abc","reason":"Game is over"}}}"#;
        assert!(matches!(parse(refused).unwrap(), Incoming::Refused(_)));

        let error = r#"{"Ok":{"Error":"Auth failed"}}"#;
        assert!(
            matches!(parse(error).unwrap(), Incoming::Error(reason) if reason == "Auth failed")
        );

        let unknown = r#"{"Ok":{"UserStatus":{"status":"Online","user":null,"username":"x"}}}"#;
        assert!(matches!(parse(unknown).unwrap(), Incoming::Other(tag) if tag == "UserStatus"));

        let ping = r#"{"Ok":{"Ping":{"nonce":7,"value":42.0}}}"#;
        assert!(matches!(parse(ping).unwrap(), Incoming::Other(tag) if tag == "Ping"));

        let heartbeat = r#"{"Ok":{"Game":{"Heartbeat":{}}}}"#;
        assert!(
            matches!(parse(heartbeat).unwrap(), Incoming::Other(tag) if tag == "Game.Heartbeat")
        );
    }

    #[test]
    fn uhp_game_string_for_a_new_game() {
        assert_eq!(game(&[]).uhp_game_string(), "Base+MLP;NotStarted;White[1]");
    }

    #[test]
    fn uhp_game_string_with_moves() {
        let mut game = game(&[("wS1", ""), ("bG1", "-wS1"), ("wA1", "wS1/")]);
        game.game_type = "Base".to_string();
        assert_eq!(
            game.uhp_game_string(),
            "Base;InProgress;Black[2];wS1;bG1 -wS1;wA1 wS1/"
        );
    }

    #[test]
    fn uhp_game_string_with_a_pass() {
        assert_eq!(
            game(&[("wS1", ""), ("bG1", "-wS1"), ("pass", "")]).uhp_game_string(),
            "Base+MLP;InProgress;Black[2];wS1;bG1 -wS1;pass"
        );
    }

    #[test]
    fn position_key_changes_with_every_move() {
        let before = game(&[("wS1", "")]);
        let after = game(&[("wS1", ""), ("bG1", "-wS1")]);
        assert_eq!(before.position_key(), game(&[("wS1", "")]).position_key());
        assert_ne!(before.position_key(), after.position_key());
    }

    #[test]
    fn only_unfinished_games_on_our_turn_are_ours_to_play() {
        let mut game = game(&[]);
        assert!(game.is_turn_of("me"));
        assert!(!game.is_turn_of("them"));
        game.finished = true;
        assert!(!game.is_turn_of("me"));
    }

    #[test]
    fn a_ready_game_is_ours_to_play_once_started() {
        let mut game = game(&[]);
        game.game_start = "Ready".to_string();
        game.game_status = json!("NotStarted");
        assert!(!game.is_turn_of("me"));
        game.game_status = json!("InProgress");
        assert!(game.is_turn_of("me"));
    }

    #[test]
    fn parses_a_finished_game() {
        let text = json!({"Ok": {"Game": {"Urgent": [{
            "game_id": "abc", "current_player_id": "me", "finished": true,
            "game_type": "MLP", "history": [["wS1", ""]], "turn": 1,
            "game_start": "Ready", "game_status": {"Finished": {"Winner": "White"}}
        }]}}})
        .to_string();
        let Incoming::Games(games) = parse(&text).unwrap() else {
            panic!("expected Games");
        };
        assert!(!games[0].is_turn_of("me"));
    }
}
