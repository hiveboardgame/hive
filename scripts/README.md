# Database Scripts

This directory contains database management scripts for data analysis, seeding, and maintenance. These scripts provide powerful tools for analyzing game data, managing test data, and generating reports.

## Quick Start

```sh
# Run any script from the project root
cargo run --bin script <command>

# Or from the scripts directory
cd scripts
cargo run --bin script <command>
```

## Available Commands

### Game Statistics (`game-stats`)
Analyzes game mechanics and tactical complexity for AI training and game balance research.

**Purpose**: Studies move complexity and board states for tactical analysis
**Output**: `game_statistics.csv` with turn-by-turn analysis of available moves and spawns

**Use Cases**:
- AI training data generation
- Game balance research
- Tactical analysis and complexity studies
- Educational content creation
- Game engine optimization

**Features**:
- Supports sampling for large datasets (`--sample-size`)
- Optional bot filtering (`--no-bots`)
- Analyzes every turn of each game to measure decision complexity
- Memory-efficient processing with temporary files

**Example**:
```sh
# Analyze 1000 random games, excluding bots
cargo run --bin script game-stats --sample-size 1000 --no-bots

# Analyze all games including bots
cargo run --bin script game-stats
```

### Games Report (`games-report`)
Analyzes player performance and game outcomes for business intelligence and player tracking.

**Purpose**: Studies player ratings, outcomes, and performance metrics
**Output**: `games_report.csv` with player-focused data including ratings, usernames, and game results

**Use Cases**:
- Player performance tracking
- Tournament analysis
- Rating system validation
- Business metrics and analytics
- Player progression studies

**Features**:
- Always excludes bot games
- Includes rating certainty analysis (rankable, provisional, clueless)
- Provides tournament and game metadata
- Fast processing with direct database queries

**Example**:
```sh
# Generate comprehensive games report
cargo run --bin script games-report
```

### Database Seeding (`seed`)
Creates realistic test data for development and testing.

**Purpose**: Populates database with test users and games for development/testing

**Features**:
- Creates configurable number of test users
- Generates realistic games with proper game mechanics
- Supports cleanup of test data (`--cleanup` flag)
- Uses database transactions for atomicity
- Implements correct Hive game rules (queen placement, move validation)

**Example**:
```sh
# Create 50 test users with 20 games each
cargo run --bin script seed --users 50 --games-per-user 20

# Clean up all test data
cargo run --bin script seed --cleanup
```

### User Management

#### List Users (`list-users`)
Lists all users in the database with their creation dates.

**Example**:
```sh
cargo run --bin script list-users
```

#### Cleanup (`cleanup`)
Removes test data including users, games, and ratings.

**Example**:
```sh
cargo run --bin script cleanup
```

## Output Files

### Game Statistics CSV (`game_statistics.csv`)
Contains tactical analysis data with the following structure:
```csv
nanoid,total_turns,moves_turn_0,spawns_turn_0,moves_turn_1,spawns_turn_1,...
game123,15,3,4,2,3,1,2,0,1,...
```

**Columns**:
- `nanoid`: Unique game identifier
- `total_turns`: Total number of turns in the game
- `moves_turn_N`: Number of available moves for turn N
- `spawns_turn_N`: Number of available spawn positions for turn N

### Games Report CSV (`games_report.csv`)
Contains player performance data with the following structure:
```csv
game_nanoid,result,white_player_username,black_player_username,white_elo,black_elo,white_elo_deviation,black_elo_deviation,white_rating_certainty,black_rating_certainty,time_control_category,tournament_game,tournament_id,game_created_at
```

**Columns**:
- `game_nanoid`: Unique game identifier
- `result`: Game outcome (White Wins, Black Wins, Draw, etc.)
- `white_player_username`, `black_player_username`: Player usernames
- `white_elo`, `black_elo`: Player ELO ratings
- `white_elo_deviation`, `black_elo_deviation`: Rating deviations
- `white_rating_certainty`, `black_rating_certainty`: Rating certainty (Rankable, Provisional, Clueless)
- `time_control_category`: Game speed category
- `tournament_game`: Whether it's a tournament game
- `tournament_id`: Tournament identifier (if applicable)
- `game_created_at`: Game creation timestamp

## Command Line Options

### Game Statistics Options
- `--database-url <URL>`: Override DATABASE_URL environment variable
- `--sample-size <N>`: Number of games to sample (random selection)
- `--no-bots`: Exclude games from bot users

### Games Report Options
- `--database-url <URL>`: Override DATABASE_URL environment variable

### Seeding Options
- `--users <N>`: Number of test users to create (default: 20)
- `--games-per-user <N>`: Number of games each user should play (default: 15)
- `--cleanup`: Clean up test data instead of seeding
- `--database-url <URL>`: Override DATABASE_URL environment variable

## Environment Variables

- `DATABASE_URL`: PostgreSQL connection string (required)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`, `warn`)

## Technical Details

### Error Handling
All scripts use `anyhow` for comprehensive error handling with detailed context messages.

### Database Transactions
The seeding process uses database transactions to ensure atomicity - either all operations succeed or none are applied.

### Memory Management
Large datasets are processed efficiently using:
- Temporary files for CSV writing
- Streaming data processing
- Optional sampling for very large datasets

### Game Logic
The seeding process implements correct Hive game rules:
- Queen placement only on turns 2-4
- No piece movement before queen placement
- Valid move/spawn selection using game engine
- Proper game state management

## Development

### Running Tests
```sh
cargo test --package script
```

### Code Quality
The scripts follow Rust best practices:
- Self-documenting function names
- Comprehensive error handling
- Clean separation of concerns
- Proper resource management

### Adding New Scripts
1. Create a new module in `src/`
2. Add the module to `src/main.rs`
3. Add the command to the CLI enum
4. Implement the command handler
5. Add tests and documentation