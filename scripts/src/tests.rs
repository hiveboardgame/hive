#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::rng;

    #[test]
    fn test_sample_games_randomly() {
        use crate::game_stats::sample_games_randomly;
        use db_lib::models::Game;
        use uuid::Uuid;
        use chrono::Utc;

        // Create mock games
        let games = vec![
            Game {
                nanoid: "test1".to_string(),
                current_player_id: Uuid::new_v4(),
                black_id: Uuid::new_v4(),
                finished: true,
                game_status: "Finished".to_string(),
                game_type: "MLP".to_string(),
                history: "Qd1;Qd8".to_string(),
                game_control_history: String::new(),
                rated: true,
                tournament_queen_rule: false,
                turn: 2,
                white_id: Uuid::new_v4(),
                white_rating: None,
                black_rating: None,
                white_rating_change: None,
                black_rating_change: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                time_mode: "RealTime".to_string(),
                time_base: Some(180),
                time_increment: Some(0),
                last_interaction: Some(Utc::now()),
                black_time_left: Some(180_000_000_000),
                white_time_left: Some(180_000_000_000),
                speed: "Blitz".to_string(),
                hashes: vec![],
                conclusion: "Winner(White)".to_string(),
                tournament_id: None,
                tournament_game_result: "Unknown".to_string(),
                game_start: "Moves".to_string(),
                move_times: vec![Some(1_000_000_000), Some(1_000_000_000)],
            },
            Game {
                nanoid: "test2".to_string(),
                current_player_id: Uuid::new_v4(),
                black_id: Uuid::new_v4(),
                finished: true,
                game_status: "Finished".to_string(),
                game_type: "MLP".to_string(),
                history: "Qd1;Qd8".to_string(),
                game_control_history: String::new(),
                rated: true,
                tournament_queen_rule: false,
                turn: 2,
                white_id: Uuid::new_v4(),
                white_rating: None,
                black_rating: None,
                white_rating_change: None,
                black_rating_change: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                time_mode: "RealTime".to_string(),
                time_base: Some(180),
                time_increment: Some(0),
                last_interaction: Some(Utc::now()),
                black_time_left: Some(180_000_000_000),
                white_time_left: Some(180_000_000_000),
                speed: "Blitz".to_string(),
                hashes: vec![],
                conclusion: "Winner(Black)".to_string(),
                tournament_id: None,
                tournament_game_result: "Unknown".to_string(),
                game_start: "Moves".to_string(),
                move_times: vec![Some(1_000_000_000), Some(1_000_000_000)],
            },
        ];

        // Test sampling with size equal to total
        let sampled = sample_games_randomly(&games, 2);
        assert_eq!(sampled.len(), 2);

        // Test sampling with size greater than total
        let sampled = sample_games_randomly(&games, 5);
        assert_eq!(sampled.len(), 2);

        // Test sampling with size less than total
        let sampled = sample_games_randomly(&games, 1);
        assert_eq!(sampled.len(), 1);
    }

    #[test]
    fn test_get_rating_certainty() {
        use crate::games_report::get_rating_certainty;
        use shared_types::RANKABLE_DEVIATION;

        // Test rankable rating
        assert_eq!(get_rating_certainty(RANKABLE_DEVIATION), "Rankable");
        assert_eq!(get_rating_certainty(50.0), "Rankable");

        // Test provisional rating
        assert_eq!(get_rating_certainty(150.0), "Provisional");
        assert_eq!(get_rating_certainty(200.0), "Provisional");

        // Test clueless rating
        assert_eq!(get_rating_certainty(250.0), "Clueless");
        assert_eq!(get_rating_certainty(500.0), "Clueless");
    }

    #[test]
    fn test_format_result() {
        use crate::games_report::format_result;

        assert_eq!(format_result("Winner(White)"), "White Wins");
        assert_eq!(format_result("Winner(Black)"), "Black Wins");
        assert_eq!(format_result("Draw"), "Draw");
        assert_eq!(format_result("Resignation(White)"), "White Resigns");
        assert_eq!(format_result("Resignation(Black)"), "Black Resigns");
        assert_eq!(format_result("Timeout(White)"), "White Timeout");
        assert_eq!(format_result("Timeout(Black)"), "Black Timeout");
        assert_eq!(format_result("Unknown"), "Unknown");
    }

    #[test]
    fn test_get_random_game_speed() {
        use crate::seed::get_random_game_speed;
        use shared_types::GameSpeed;

        let speed = get_random_game_speed();
        
        // Should be one of the valid game speeds
        match speed {
            GameSpeed::Bullet | GameSpeed::Blitz | GameSpeed::Rapid | 
            GameSpeed::Classic | GameSpeed::Correspondence => {},
            _ => panic!("Invalid game speed returned: {:?}", speed),
        }
    }

    #[test]
    fn test_retry_operation_success() {
        use crate::common::retry_operation;
        use std::future::Future;
        use std::pin::Pin;

        let operation = || {
            Box::pin(async move {
                Ok::<i32, anyhow::Error>(42)
            }) as Pin<Box<dyn Future<Output = Result<i32>> + Send>>
        };

        let result = futures::executor::block_on(retry_operation(operation, 3, 10));
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_retry_operation_failure() {
        use crate::common::retry_operation;
        use std::future::Future;
        use std::pin::Pin;

        let operation = || {
            Box::pin(async move {
                Err::<i32, anyhow::Error>(anyhow::anyhow!("Always fails"))
            }) as Pin<Box<dyn Future<Output = Result<i32>> + Send>>
        };

        let result = futures::executor::block_on(retry_operation(operation, 2, 1));
        assert!(result.is_err());
    }

    #[test]
    fn test_anyhow_error_handling() {
        let result: Result<()> = Err(anyhow::anyhow!("Test error"));
        assert!(result.is_err());
    }
}
