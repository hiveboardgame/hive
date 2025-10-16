# Hive Database Scripts

A comprehensive CLI tool for managing the Hive database with multiple subcommands for different tasks.

## Features

The tool provides several subcommands for database management:

- **Seed**: Populate the database with test data for development
- **ListUsers**: List all users in the database
- **Cleanup**: Remove test data from the database
- **GameStats**: Generate game statistics CSV file
- **GamesReport**: Generate comprehensive games report CSV file

## Prerequisites

- PostgreSQL database running
- `.env` file in the parent directory with `DATABASE_URL` set
- Rust toolchain installed

## Usage

### General Help
```bash
cd scripts
cargo run -- --help
```

### Seed Database
```bash
# Use default values (20 users, 15 games each)
cargo run -- seed

# Customize the number of users and games
cargo run -- seed --users 50 --games-per-user 25

# Use a custom database URL
cargo run -- seed --database-url "postgresql://user:pass@localhost:5432/hive_db"
```

### List Users
```bash
# List all users in the database
cargo run -- list-users

# Use a custom database URL
cargo run -- list-users --database-url "postgresql://user:pass@localhost:5432/hive_db"
```

### Cleanup Test Data
```bash
# Remove all test data
cargo run -- cleanup

# Use a custom database URL
cargo run -- cleanup --database-url "postgresql://user:pass@localhost:5432/hive_db"
```

### Generate Game Statistics
```bash
# Generate game statistics CSV file (all games)
cargo run -- game-stats

# Sample 1000 random games
cargo run -- game-stats --sample-size 1000

# Exclude bot games
cargo run -- game-stats --no-bots

# Sample 500 games excluding bots
cargo run -- game-stats --sample-size 500 --no-bots

# Use a custom database URL
cargo run -- game-stats --database-url "postgresql://user:pass@localhost:5432/hive_db"
```

### Generate Games Report
```bash
# Generate comprehensive games report CSV file
cargo run -- games-report

# Use a custom database URL
cargo run -- games-report --database-url "postgresql://user:pass@localhost:5432/hive_db"
```

## Command Options

### Seed Command
- `-u, --users <USERS>`: Number of test users to create (default: 20)
- `-g, --games-per-user <GAMES_PER_USER>`: Number of games each user should play (default: 15)
- `--database-url <DATABASE_URL>`: Database URL (overrides DATABASE_URL env var)

### ListUsers Command
- `--database-url <DATABASE_URL>`: Database URL (overrides DATABASE_URL env var)

### Cleanup Command
- `--database-url <DATABASE_URL>`: Database URL (overrides DATABASE_URL env var)

### GameStats Command
- `--database-url <DATABASE_URL>`: Database URL (overrides DATABASE_URL env var)
- `-s, --sample-size <SAMPLE_SIZE>`: Number of games to randomly sample (if not specified, analyzes all games)
- `--no-bots`: Exclude games from users with bot == true

### GamesReport Command
- `--database-url <DATABASE_URL>`: Database URL (overrides DATABASE_URL env var)

## Environment Variables

Make sure your `.env` file contains:
```
DATABASE_URL=postgresql://username:password@localhost:5432/hive_db
```

## Project Structure

```
scripts/
├── src/
│   ├── main.rs          # CLI entry point and command definitions
│   ├── mod.rs           # Module declarations
│   ├── seed.rs          # Database seeding functionality
│   ├── users.rs         # User management functionality
│   ├── game_stats.rs    # Game statistics generation
│   └── games_report.rs  # Comprehensive games report generation
├── Cargo.toml           # Dependencies and build configuration
├── run_seed.sh          # Legacy shell script (can be removed)
└── README.md            # This file
```

## Adding New Scripts

To add a new script:

1. Create a new module file in `src/` (e.g., `src/tournaments.rs`)
2. Add the module to `src/mod.rs`
3. Add a new variant to the `Commands` enum in `main.rs`
4. Implement the command logic in your module
5. Add the command handler in the main function

### Example: Adding a Tournament Script

```rust
// In src/tournaments.rs
pub async fn list_tournaments(database_url: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Implementation here
    Ok(())
}

// In src/mod.rs
pub mod tournaments;

// In main.rs - add to Commands enum
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// List all tournaments in the database
    ListTournaments {
        /// Database URL (overrides DATABASE_URL env var)
        #[arg(long)]
        database_url: Option<String>,
    },
}

// In main.rs - add to match statement
match cli.command {
    // ... existing matches ...
    Commands::ListTournaments { database_url } => {
        tournaments::list_tournaments(database_url).await?;
    }
}
```

## Testing the Leaderboard

After running the seed command:
1. Start the Hive application
2. Navigate to the leaderboard page
3. You should see the top 10 players for each game speed
4. Log in as one of the test users to see their rating information
5. Test the "rankable vs non-rankable" rating display

## Game Statistics

The `game-stats` command analyzes games in the database and generates a CSV file with detailed statistics:

- **File Output**: `game_statistics.csv` in the current directory
- **Data Included**: 
  - Game nanoid and total turns
  - Number of available moves for each turn
  - Number of available spawn positions for each turn
  - Analysis of game complexity and decision points
- **Filtering Options**:
  - **Sampling**: Randomly select a subset of games for faster analysis
  - **Bot Exclusion**: Exclude games involving bot players
- **Use Cases**:
  - Game balance analysis
  - AI training data
  - Gameplay pattern research
  - Performance optimization insights
  - Spawn point availability analysis
  - Opening theory development

The CSV file can be imported into spreadsheet applications or data analysis tools for further processing.

## Games Report

The `games-report` command generates a comprehensive CSV report of all games (excluding bot games) with detailed player and game information:

- **File Output**: `games_report.csv` in the current directory
- **Data Included**:
  - Game nanoid and result
  - Player usernames (white and black)
  - ELO ratings for both players
  - ELO deviation (rating certainty: Rankable/Provisional/Clueless)
  - Time control category (Bullet/Blitz/Rapid/Classic/Correspondence)
  - Tournament information (whether it's a tournament game and tournament ID)
  - Game creation timestamp
- **Use Cases**:
  - Player performance analysis
  - Rating system evaluation
  - Tournament statistics
  - Game balance research
  - Competitive analysis

The report excludes bot games and provides Excel-friendly formatting for easy analysis and visualization.

## Customization

You can modify the constants in the seed module:
- `NUM_USERS`: Number of test users to create
- `GAMES_PER_USER`: Number of games each user plays
- `GAME_SPEEDS`: Which game speeds to include

## Cleanup

To remove test data, use the cleanup command:
```bash
cargo run -- cleanup
```

This will:
- Delete all users with usernames starting with "testuser"
- Automatically remove associated games and ratings due to foreign key constraints

## Troubleshooting

### Linking Errors
If you get PostgreSQL linking errors during development, this is normal. The CLI structure can be verified with:
```bash
cargo check
```

### Database Connection Issues
- Ensure PostgreSQL is running
- Check that `DATABASE_URL` is correct
- Verify database permissions
- Make sure the database exists

### Permission Errors
- Ensure your database user has the necessary permissions
- Check that the database exists and is accessible
