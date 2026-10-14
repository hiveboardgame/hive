use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hive_lib::{History, State};
use puzzler::{
    corpus,
    game::Game,
    line,
    mine::{mine_game, MineConfig, MineStats},
    notation,
    prove::Prover,
    push,
};
use rayon::prelude::*;
use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::Mutex,
    time::Instant,
};

#[derive(Parser)]
#[command(about = "Finds, checks and publishes Hive puzzles")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Imports the Hive Puzzles Vol. 1 book from `pdfinfo -url` and `pdftotext -layout` output.
    Book {
        urls: PathBuf,
        text: PathBuf,
        #[arg(long, default_value = "book.jsonl")]
        out: PathBuf,
        #[arg(long, default_value_t = 9)]
        max_plies: u32,
        /// Node budget per page; a page that needs more is skipped.
        #[arg(long, default_value_t = 300_000_000)]
        budget: u64,
    },
    /// Drops wins in one that only fill the Queen's last gap (keeping missed wins, Pillbug throws
    /// and Mosquito tricks) and keeps `--warm-ups` of the plainest as an easy set.
    Filter {
        input: PathBuf,
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        warm_ups: usize,
        #[arg(long, default_value_t = 800.0)]
        warm_up_rating: f64,
    },
    /// Sends a JSONL file of puzzles to a hivegame server (PUZZLE_IMPORT_TOKEN must match).
    Push {
        file: PathBuf,
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
        /// Defaults to the PUZZLE_IMPORT_TOKEN environment variable.
        #[arg(long, env = "PUZZLE_IMPORT_TOKEN", hide_env_values = true)]
        token: String,
    },
    /// Scans the hivegame export for forced wins and writes puzzles as JSON lines.
    Mine {
        corpus: PathBuf,
        #[arg(long, default_value = "puzzles.jsonl")]
        out: PathBuf,
        /// Only games of this type (e.g. Base+MLP); all types when absent.
        #[arg(long)]
        game_type: Option<String>,
        /// Stop after this many games (after skipping ones already done).
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value_t = 5)]
        max_plies: u32,
        #[arg(long, default_value_t = 20)]
        window: usize,
        #[arg(long, default_value_t = 200_000)]
        budget: u64,
        /// Depth of the cheap scan over the rest of the game (missed wins).
        #[arg(long, default_value_t = 3)]
        early_plies: u32,
        /// Depth tried just before the earliest found win; 0 disables it.
        #[arg(long, default_value_t = 7)]
        extend_plies: u32,
        #[arg(long, default_value_t = 1_500_000)]
        extend_budget: u64,
    },
    /// Proves one position (a UHP gamestring) and prints the puzzle line.
    Prove {
        gamestring: String,
        #[arg(long, default_value_t = 9)]
        max_plies: u32,
        #[arg(long, default_value_t = 500_000_000)]
        budget: u64,
    },
    /// Proves every line of a philanthus-style puzzle CSV (`gamestring plies solution`).
    Bench {
        csv: PathBuf,
        #[arg(long, default_value_t = 100)]
        count: usize,
        #[arg(long, default_value_t = 50_000_000)]
        budget: u64,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    match Cli::parse().command {
        Command::Bench { csv, count, budget } => bench(&csv, count, budget),
        Command::Filter {
            input,
            out,
            warm_ups,
            warm_up_rating,
        } => filter(&input, &out, warm_ups, warm_up_rating),
        Command::Push {
            file,
            base_url,
            token,
        } => {
            let report = push::push(&file, &base_url, &token)?;
            for rejection in &report.rejections {
                println!("line {}: {}", rejection.line, rejection.reason);
            }
            println!(
                "{} inserted, {} already there, {} rejected",
                report.inserted, report.duplicate, report.rejected
            );
            Ok(())
        }
        Command::Book {
            urls,
            text,
            out,
            max_plies,
            budget,
        } => import_book(&urls, &text, &out, max_plies, budget),
        Command::Mine {
            corpus,
            out,
            game_type,
            limit,
            max_plies,
            window,
            budget,
            early_plies,
            extend_plies,
            extend_budget,
        } => mine(
            &corpus,
            &out,
            game_type.as_deref(),
            limit,
            MineConfig {
                max_plies,
                window,
                budget,
                early_plies,
                extend_plies,
                extend_budget,
                ..MineConfig::default()
            },
        ),
        Command::Prove {
            gamestring,
            max_plies,
            budget,
        } => prove(&gamestring, max_plies, budget),
    }
}

fn filter(input: &PathBuf, out: &PathBuf, warm_ups: usize, warm_up_rating: f64) -> Result<()> {
    use puzzler::select::{retag_quiet, verdict, warm_ups as pick_warm_ups, Verdict};
    use shared_types::PuzzleImport;
    use std::io::Write;
    let text = std::fs::read_to_string(input).with_context(|| format!("reading {input:?}"))?;
    let mut kept = Vec::new();
    let mut plain = Vec::new();
    let mut retagged = 0;
    for (number, line) in text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let mut puzzle: PuzzleImport =
            serde_json::from_str(line).with_context(|| format!("line {}", number + 1))?;
        if retag_quiet(&mut puzzle).with_context(|| format!("line {}", number + 1))? {
            retagged += 1;
        }
        match verdict(&mut puzzle).with_context(|| format!("line {}", number + 1))? {
            Verdict::Keep => kept.push(puzzle),
            Verdict::Plain => plain.push(puzzle),
        }
    }
    let dropped = plain.len();
    let easy = pick_warm_ups(plain, warm_ups, warm_up_rating);
    let mut writer = std::io::BufWriter::new(
        std::fs::File::create(out).with_context(|| format!("creating {out:?}"))?,
    );
    for puzzle in kept.iter().chain(&easy) {
        writeln!(writer, "{}", serde_json::to_string(puzzle)?)?;
    }
    writer.flush()?;
    println!(
        "{} kept, {} plain wins in one dropped, {} of them kept as warm-ups, {retagged} quiet-move tags changed",
        kept.len(),
        dropped - easy.len(),
        easy.len()
    );
    Ok(())
}

fn bench(csv: &PathBuf, count: usize, budget: u64) -> Result<()> {
    let text = std::fs::read_to_string(csv).with_context(|| format!("reading {csv:?}"))?;
    let mut prover = Prover::new(budget);
    let (mut solved, mut shorter, mut missed, mut aborted, mut total) = (0, 0, 0, 0, 0);
    let mut nodes = 0u64;
    let start = Instant::now();
    for line in text.lines().take(count) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(index) = tokens.iter().position(|token| token.parse::<u32>().is_ok()) else {
            continue;
        };
        let expected: u32 = tokens[index].parse()?;
        let gamestring = tokens[..index].join(" ");
        let history = History::from_uhp_str(&gamestring)?;
        let state = State::new_from_history(&history)?;
        let mut game = Game::from_state(&state);
        total += 1;
        prover.reset_budget(budget);
        match prover.shortest_win(&mut game, expected) {
            Ok(Some((plies, _))) if plies == expected => solved += 1,
            Ok(Some(_)) => shorter += 1,
            Ok(None) => {
                missed += 1;
                tracing::warn!(line = total, expected, "no forced win found");
            }
            Err(_) => aborted += 1,
        }
        nodes += prover.nodes;
    }
    let seconds = start.elapsed().as_secs_f64();
    println!(
        "{total} puzzles: {solved} exact, {shorter} shorter, {missed} missed, {aborted} over budget; \
         {nodes} nodes in {seconds:.1}s ({:.0} knodes/s)",
        nodes as f64 / seconds / 1000.0
    );
    Ok(())
}

fn prove(gamestring: &str, max_plies: u32, budget: u64) -> Result<()> {
    let history = History::from_uhp_str(gamestring)?;
    let state = State::new_from_history(&history)?;
    let game = Game::from_state(&state);
    let mut prover = Prover::new(budget);
    let start = Instant::now();
    let Some((plies, first)) = prover
        .shortest_win(&mut game.clone(), max_plies)
        .map_err(|_| anyhow::anyhow!("over budget"))?
    else {
        println!(
            "no forced win within {max_plies} plies ({} nodes)",
            prover.nodes
        );
        return Ok(());
    };
    println!(
        "{} wins in {plies} plies, first {} ({} nodes, {:.2}s)",
        state.turn_color,
        notation::notation(&state, &first),
        prover.nodes,
        start.elapsed().as_secs_f64()
    );
    match line::solve(&mut prover, &game, plies).map_err(|_| anyhow::anyhow!("over budget"))? {
        Ok(line) => {
            for (number, actions) in line.lines.iter().enumerate() {
                let mut replay = state.clone();
                let mut tokens = Vec::new();
                for action in actions {
                    tokens.push(notation::notation(&replay, action));
                    notation::play(&mut replay, action)?;
                }
                println!(
                    "  line {number}: {}  => {}",
                    tokens.join("; "),
                    replay.game_status
                );
            }
        }
        Err(rejection) => println!("  rejected: {rejection:?}"),
    }
    Ok(())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn mine(
    corpus_path: &std::path::Path,
    out: &PathBuf,
    game_type: Option<&str>,
    limit: Option<usize>,
    config: MineConfig,
) -> Result<()> {
    let done_path = out.with_extension("done");
    let done: HashSet<String> = match std::fs::File::open(&done_path) {
        Ok(file) => BufReader::new(file).lines().map_while(Result::ok).collect(),
        Err(_) => HashSet::new(),
    };
    let games: Vec<_> = corpus::read(corpus_path)?
        .into_iter()
        .filter(|game| game_type.is_none_or(|wanted| game.game_type == wanted))
        .filter(|game| !done.contains(&game.nanoid))
        .take(limit.unwrap_or(usize::MAX))
        .collect();
    tracing::info!(games = games.len(), skipped = done.len(), ?config, "mining");

    let open = |path: &PathBuf| OpenOptions::new().create(true).append(true).open(path);
    let output = Mutex::new((open(out)?, open(&done_path)?, MineStats::default(), 0usize));
    let start = Instant::now();
    let total = games.len();
    games.par_iter().for_each_init(
        || Prover::new(config.budget),
        |prover, game| {
            let (puzzles, stats) = mine_game(prover, game, &config, VERSION);
            let Ok(mut guard) = output.lock() else {
                return;
            };
            let (puzzle_file, done_file, totals, finished) = &mut *guard;
            for puzzle in &puzzles {
                if let Ok(line) = serde_json::to_string(puzzle) {
                    let _ = writeln!(puzzle_file, "{line}");
                }
            }
            let _ = writeln!(done_file, "{}", game.nanoid);
            totals.add(&stats);
            *finished += 1;
            if *finished % 200 == 0 || *finished == total {
                let seconds = start.elapsed().as_secs_f64();
                tracing::info!(
                    games = *finished,
                    of = total,
                    puzzles = totals.puzzles,
                    positions = totals.positions,
                    wins = totals.wins,
                    ambiguous = totals.ambiguous,
                    plain = totals.plain,
                    over_budget = totals.over_budget,
                    games_per_second = format!("{:.1}", *finished as f64 / seconds),
                    "progress"
                );
            }
        },
    );
    Ok(())
}

fn import_book(
    urls: &PathBuf,
    text: &PathBuf,
    out: &PathBuf,
    max_plies: u32,
    budget: u64,
) -> Result<()> {
    use puzzler::book::{import_page, read_pages, Outcome};
    let pages = read_pages(
        &std::fs::read_to_string(urls).with_context(|| format!("reading {urls:?}"))?,
        &std::fs::read_to_string(text).with_context(|| format!("reading {text:?}"))?,
    );
    let mut prover = Prover::new(2_000_000_000);
    let mut output = std::fs::File::create(out)?;
    let mut imported = 0;
    for page in &pages {
        match import_page(page, &mut prover, max_plies, budget) {
            Ok(Outcome::Puzzle(puzzle)) => {
                writeln!(output, "{}", serde_json::to_string(&puzzle)?)?;
                imported += 1;
                println!("#{:2} ok      {}", page.number, page.authors.join(", "));
            }
            Ok(Outcome::Skipped(reason)) => println!("#{:2} skipped {reason}", page.number),
            Err(error) => println!("#{:2} failed  {error:#}", page.number),
        }
    }
    println!("{imported} of {} pages imported into {out:?}", pages.len());
    Ok(())
}
