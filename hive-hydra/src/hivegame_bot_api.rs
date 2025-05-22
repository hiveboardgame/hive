use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing::debug;

const API_TIMEOUT: u64 = 10; // 10 seconds timeout for API calls

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    Request(#[from] ReqwestError),
    #[error("API error: {status_code} - {message}")]
    Server {
        status_code: reqwest::StatusCode,
        message: String,
    },
    #[error("JSON error: {0}")]
    Json(#[from] JsonError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HiveGame {
    #[serde(rename = "id")]
    pub game_id: String,
    #[serde(rename = "time_base")]
    pub time: Option<i32>,
    #[serde(default)]
    pub opponent_username: String,
    pub game_type: String,
    pub game_status: String,
    #[serde(default)]
    pub player_turn: String,
    #[serde(rename = "history", default)]
    pub moves: String,
    // Additional fields from API response - we can ignore most of these
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nanoid: Option<String>,
    #[serde(default)]
    pub black_id: String,
    #[serde(default)]
    pub white_id: String,
    #[serde(default)]
    pub current_player_id: String,
}

#[derive(Debug, Serialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponseData {
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    data: AuthResponseData,
}

#[derive(Debug, Deserialize)]
struct Challenge {
    challenge_id: String,
}

#[derive(Debug, Deserialize)]
struct ChallengesData {
    challenges: Vec<Challenge>,
}

#[derive(Debug, Deserialize)]
struct ChallengesResponse {
    data: ChallengesData,
}

#[derive(Debug, Deserialize)]
struct GamesData {
    games: Vec<HiveGame>,
}

#[derive(Debug, Deserialize)]
struct GamesResponse {
    data: GamesData,
}

impl HiveGame {
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.game_id.hash(&mut hasher);
        self.game_type.hash(&mut hasher);
        self.game_status.hash(&mut hasher);
        self.player_turn.hash(&mut hasher);
        self.moves.hash(&mut hasher);
        hasher.finish()
    }

    pub fn game_string(&self) -> String {
        // If moves is empty, don't include a trailing semicolon
        if self.moves.is_empty() {
            return format!("{};{};{}", self.game_type, self.game_status, "White[1]");
        }

        // First create an owned string with spaces before semicolons removed
        let moves_without_spaces = self.moves.replace(" ;", ";");

        // Then trim trailing semicolons and spaces
        let cleaned_moves = moves_without_spaces.trim_end_matches(";");

        debug!(
            "Original Moves: [{}], Cleaned: [{}]",
            self.moves, cleaned_moves
        );

        format!(
            "{};{};{};{}",
            self.game_type, self.game_status, self.player_turn, cleaned_moves
        )
    }
}

#[derive(Debug, Serialize)]
struct PlayMove {
    game_id: String,
    piece_pos: String,
}

pub struct HiveGameApi {
    client: Client,
    base_url: String,
}

impl HiveGameApi {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(API_TIMEOUT))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Authenticate with email and password to get a token
    pub async fn auth(&self, email: &str, password: &str) -> Result<String, ApiError> {
        let url = format!("{}/api/v1/auth/token", self.base_url);

        let auth_request = AuthRequest {
            email: email.to_string(),
            password: password.to_string(),
        };

        let response = self.client.post(&url).json(&auth_request).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Server {
                status_code: status,
                message: response.text().await.unwrap_or_default(),
            });
        }

        let response_text = response.text().await?;

        // Parse the response JSON from the saved text
        let auth_response: AuthResponse = serde_json::from_str(&response_text)?;
        Ok(auth_response.data.token)
    }

    /// Get all active games for a bot
    /// Returns a vector of HiveGame
    pub async fn get_games(&self, token: &str) -> Result<Vec<HiveGame>, ApiError> {
        let url = format!("{}/api/v1/bot/games/pending", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Server {
                status_code: status,
                message: response.text().await.unwrap_or_default(),
            });
        }

        let response_text = response.text().await?;

        // Parse the response JSON using the nested structure
        let games_response: GamesResponse = serde_json::from_str(&response_text)?;

        // Extract just the games array from the nested structure
        Ok(games_response.data.games)
    }

    /// Send a move to the game
    pub async fn play_move(
        &self,
        game_id: &str,
        move_notation: &str,
        token: &str,
    ) -> Result<(), ApiError> {
        let url = format!("{}/api/v1/bot/games/play", self.base_url);

        let payload = PlayMove {
            game_id: game_id.to_string(),
            piece_pos: move_notation.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Server {
                status_code: status,
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Get all challenges for a bot
    /// Returns a vector of challenge IDs
    pub async fn challenges(&self, token: &str) -> Result<Vec<String>, ApiError> {
        let url = format!("{}/api/v1/bot/challenges/", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Server {
                status_code: status,
                message: response.text().await.unwrap_or_default(),
            });
        }

        // Deserialize the response into our ChallengesResponse struct
        let response_json: ChallengesResponse = response.json().await?;

        // Extract challenge IDs into a vector
        let challenge_ids = response_json
            .data
            .challenges
            .iter()
            .map(|challenge| challenge.challenge_id.clone())
            .collect();

        debug!("Challenges received: {:?}", challenge_ids);

        Ok(challenge_ids)
    }

    /// Accept a challenge for a bot
    /// Takes a challenge ID and sends a request to accept it
    pub async fn accept_challenge(&self, challenge_id: &str, token: &str) -> Result<(), ApiError> {
        let url = format!(
            "{}/api/v1/bot/challenge/accept/{}",
            self.base_url, challenge_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Server {
                status_code: status,
                message: response.text().await.unwrap_or_default(),
            });
        }

        // Print response for debugging
        let response_text = response.text().await?;
        debug!(
            "Challenge acceptance response for {}: {}",
            challenge_id, response_text
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::Request;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn verify_auth_header(req: &Request, expected_key: &str) {
        let auth_header = req
            .headers
            .get(&"Authorization".parse().unwrap())
            .expect("Authorization header missing");

        let expected_value = format!("Bearer {}", expected_key);
        assert_eq!(auth_header[0], expected_value);
    }

    #[tokio::test]
    async fn test_auth() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for auth
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/token"))
            .and(body_json(json!({
                "email": "bot@example.com",
                "password": "hivegame"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "token": "test_token_123"
                },
                "success": true
            })))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let token = api.auth("bot@example.com", "hivegame").await.unwrap();

        assert_eq!(token, "test_token_123");
    }

    #[tokio::test]
    async fn test_auth_failure() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for failed auth
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/token"))
            .and(body_json(json!({
                "email": "wrong@example.com",
                "password": "wrong_password"
            })))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.auth("wrong@example.com", "wrong_password").await;

        assert!(matches!(result,
            Err(ApiError::Server {
                status_code,
                message
            }) if status_code == 401 && message == "Unauthorized"
        ));
    }

    #[tokio::test]
    async fn test_get_games() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response with multiple games
        Mock::given(method("GET"))
            .and(path("/api/v1/bot/games/pending"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "bot": "bot1@example.com",
                    "games": [
                        {
                            "id": "123",
                            "time_base": 20,
                            "opponent_username": "player1",
                            "game_type": "Base+PLM",
                            "game_status": "InProgress",
                            "player_turn": "White[3]",
                            "history": "wS1;bG1 -wS1;wA1 wS1/;bG2 /bG1"
                        },
                        {
                            "id": "456",
                            "time_base": 10,
                            "opponent_username": "player2",
                            "game_type": "Base",
                            "game_status": "InProgress",
                            "player_turn": "Black[2]",
                            "history": "bS1;wG1 -bS1;bA1 bS1/;wG2 /wG1"
                        }
                    ]
                },
                "success": true
            })))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let games = api.get_games("test_key").await.unwrap();

        // Verify we got the expected number of games
        assert_eq!(games.len(), 2);

        // Verify first game
        assert_eq!(games[0].game_id, "123");
        assert_eq!(games[0].game_type, "Base+PLM");
        assert_eq!(games[0].game_status, "InProgress");
        assert_eq!(games[0].player_turn, "White[3]");
        assert_eq!(games[0].moves, "wS1;bG1 -wS1;wA1 wS1/;bG2 /bG1");

        // Verify second game
        assert_eq!(games[1].game_id, "456");
        assert_eq!(games[1].game_type, "Base");
        assert_eq!(games[1].game_status, "InProgress");
        assert_eq!(games[1].player_turn, "Black[2]");
        assert_eq!(games[1].moves, "bS1;wG1 -bS1;bA1 bS1/;wG2 /wG1");
    }

    #[tokio::test]
    async fn test_play_move() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response
        Mock::given(method("POST"))
            .and(path("/api/v1/bot/games/play"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.play_move("123", "wS1", "test_key").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/bot/games/pending"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.get_games("test_key").await;

        assert!(matches!(result,
            Err(ApiError::Server {
                status_code,
                message
            }) if status_code == 404 && message == "Not found"
        ));
    }

    #[test]
    fn test_game_string() {
        let game = HiveGame {
            game_id: "123".to_string(),
            time: Some(20),
            opponent_username: "player1".to_string(),
            game_type: "Base".to_string(),
            game_status: "InProgress".to_string(),
            player_turn: "White[3]".to_string(),
            moves: "wS1;bG1 -wS1;wA1 wS1/;bG2 /bG1".to_string(),
            nanoid: None,
            black_id: "".to_string(),
            white_id: "".to_string(),
            current_player_id: "".to_string(),
        };

        let expected = "Base;InProgress;White[3];wS1;bG1 -wS1;wA1 wS1/;bG2 /bG1";
        assert_eq!(game.game_string(), expected);
    }

    #[tokio::test]
    async fn test_challenges() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for challenges endpoint with proper structure
        Mock::given(method("GET"))
            .and(path("/api/v1/bot/challenges/"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "bot": "bot1@example.com",
                    "challenges": [
                        {
                            "challenge_id": "qaTq1dsIi3-i",
                            "game_type": "Base+MLP"
                        },
                        {
                            "challenge_id": "abCdEfGhIj-z",
                            "game_type": "Base"
                        }
                    ]
                },
                "success": true
            })))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let challenge_ids = api.challenges("test_key").await.unwrap();

        // Verify we got the expected number of challenges
        assert_eq!(challenge_ids.len(), 2);
        assert_eq!(challenge_ids[0], "qaTq1dsIi3-i");
        assert_eq!(challenge_ids[1], "abCdEfGhIj-z");
    }

    #[tokio::test]
    async fn test_challenges_error() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for failed challenges request
        Mock::given(method("GET"))
            .and(path("/api/v1/bot/challenges/"))
            .and(|req: &Request| {
                verify_auth_header(req, "invalid_key");
                true
            })
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.challenges("invalid_key").await;

        assert!(matches!(result,
            Err(ApiError::Server {
                status_code,
                message
            }) if status_code == 401 && message == "Unauthorized"
        ));
    }

    #[tokio::test]
    async fn test_accept_challenge() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for accept challenge endpoint
        Mock::given(method("GET"))
            .and(path("/api/v1/bot/challenge/accept/qaTq1dsIi3-i"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "game_id": "789",
                    "message": "Challenge accepted"
                },
                "success": true
            })))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.accept_challenge("qaTq1dsIi3-i", "test_key").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_accept_challenge_error() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Create mock response for failed accept challenge request
        Mock::given(method("GET"))
            .and(path("/api/v1/bot/challenge/accept/invalid-id"))
            .and(|req: &Request| {
                verify_auth_header(req, "test_key");
                true
            })
            .respond_with(ResponseTemplate::new(404).set_body_string("Challenge not found"))
            .mount(&mock_server)
            .await;

        let api = HiveGameApi::new(mock_server.uri());
        let result = api.accept_challenge("invalid-id", "test_key").await;

        assert!(matches!(result,
            Err(ApiError::Server {
                status_code,
                message
            }) if status_code == 404 && message == "Challenge not found"
        ));
    }
}
