use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "hive-hydra.yaml")]
    pub config: String,
}

impl Cli {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args_os())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cli = Cli::parse_from(["program"]);
        assert_eq!(cli.config, "hive-hydra.yaml");
    }

    #[test]
    fn test_custom_config_short() {
        let cli = Cli::parse_from(["program", "-c", "custom.yaml"]);
        assert_eq!(cli.config, "custom.yaml");
    }

    #[test]
    fn test_custom_config_long() {
        let cli = Cli::parse_from(["program", "--config", "custom.yaml"]);
        assert_eq!(cli.config, "custom.yaml");
    }
}
