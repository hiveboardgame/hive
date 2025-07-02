use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub max_concurrent_processes: usize,
    pub queue_capacity: usize,
    pub base_url: String,
    pub bots: Vec<BotConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub name: String,
    pub ai_command: String,
    pub bestmove_command_args: String,
    pub email: String,
    pub password: String,
}

impl Config {
    pub fn load_from<P: AsRef<Path>>(config_path: P) -> Result<Self, ConfigError> {
        let settings = ConfigBuilder::builder()
            // 1. Default values
            .set_default("max_concurrent_processes", 5)?
            .set_default("queue_capacity", 1000)?
            .set_default("base_url", "https://hivegame.com")?
            // 2. Config file
            .add_source(File::from(config_path.as_ref()))
            .add_source(Environment::with_prefix("HIVE_HYDRA"))
            .build()?;

        // Load the main configuration
        let mut config: Config = settings.try_deserialize()?;

        // Process bot-specific environment variables
        let env_vars: HashMap<String, String> = std::env::vars().collect();
        for bot in &mut config.bots {
            let prefix = format!(
                "HIVE_HYDRA_BOT_{}_",
                bot.name.to_uppercase().replace('-', "_")
            );

            if let Some(value) = env_vars.get(&format!("{prefix}EMAIL")) {
                bot.email = value.clone();
            }
            if let Some(value) = env_vars.get(&format!("{prefix}PASSWORD")) {
                bot.password = value.clone();
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn test_env_override_with_custom_config() {
        // Clear existing env vars
        env::remove_var("HIVE_HYDRA_MAX_CONCURRENT_PROCESSES");
        env::remove_var("HIVE_HYDRA_BOT_TESTBOT1_EMAIL");
        env::remove_var("HIVE_HYDRA_BOT_TESTBOT1_PASSWORD");

        // Set test env vars
        env::set_var("HIVE_HYDRA_MAX_CONCURRENT_PROCESSES", "10");
        env::set_var("HIVE_HYDRA_BOT_TESTBOT1_EMAIL", "test_email_1");
        env::set_var("HIVE_HYDRA_BOT_TESTBOT1_PASSWORD", "test_password_1");

        // Create test config file
        let config_content = r#"
max_concurrent_processes: 5
queue_capacity: 1000
base_url: "https://hivegame.com"
bots:
  - name: testbot1
    ai_command: test_command
    bestmove_command_args: depth 1
    email: default_email1
    password: default_password1
"#;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.yaml");
        fs::write(&file_path, config_content).unwrap();

        // Load config and test
        let config = Config::load_from(&file_path).unwrap();
        assert_eq!(
            config.max_concurrent_processes, 10,
            "max_concurrent_processes should be overridden by environment variable"
        );
        assert_eq!(
            config.bots[0].email, "test_email_1",
            "bot email should be overridden by environment variable"
        );
        assert_eq!(
            config.bots[0].password, "test_password_1",
            "bot password should be overridden by environment variable"
        );

        // Cleanup
        env::remove_var("HIVE_HYDRA_MAX_CONCURRENT_PROCESSES");
        env::remove_var("HIVE_HYDRA_BOT_TESTBOT1_EMAIL");
        env::remove_var("HIVE_HYDRA_BOT_TESTBOT1_PASSWORD");
    }

    #[test]
    fn test_env_var_with_hyphen_in_bot_name() {
        // Clear existing env vars
        env::remove_var("HIVE_HYDRA_BOT_TEST_BOT_EMAIL");
        env::remove_var("HIVE_HYDRA_BOT_TEST_BOT_PASSWORD");

        // Set test env var with underscore
        env::set_var("HIVE_HYDRA_BOT_TEST_BOT_EMAIL", "test_email_1");
        env::set_var("HIVE_HYDRA_BOT_TEST_BOT_PASSWORD", "test_password_1");

        // Create test config file with hyphen in bot name
        let config_content = r#"
max_concurrent_processes: 5
queue_capacity: 1000
base_url: "https://hivegame.com"
bots:
  - name: test-bot
    ai_command: test_command
    bestmove_command_args: depth 1
    email: default_email1
    password: default_password1
"#;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.yaml");
        fs::write(&file_path, config_content).unwrap();

        // Load config and test
        let config = Config::load_from(&file_path).unwrap();
        assert_eq!(
            config.bots[0].email, "test_email_1",
            "bot email should be overridden by environment variable with underscore"
        );
        assert_eq!(
            config.bots[0].password, "test_password_1",
            "bot password should be overridden by environment variable with underscore"
        );

        // Cleanup
        env::remove_var("HIVE_HYDRA_BOT_TEST_BOT_EMAIL");
        env::remove_var("HIVE_HYDRA_BOT_TEST_BOT_PASSWORD");
    }
}
