use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

/// One row of the hivegame export (see docs/puzzles-plan.md for the query).
#[derive(Debug, Clone)]
pub struct CorpusGame {
    pub nanoid: String,
    pub game_type: String,
    pub game_status: String,
    pub conclusion: String,
    pub rated: bool,
    pub white_rating: Option<f64>,
    pub black_rating: Option<f64>,
    pub white_bot: bool,
    pub black_bot: bool,
    pub tournament_queen_rule: bool,
    pub history: String,
}

impl CorpusGame {
    pub fn involves_bot(&self) -> bool {
        self.white_bot || self.black_bot
    }

    pub fn moves(&self) -> Vec<&str> {
        self.history
            .split(';')
            .filter(|token| !token.trim().is_empty())
            .collect()
    }
}

const COLUMNS: usize = 14;

fn parse_row(line: &str) -> Result<CorpusGame> {
    // The history is the last column and holds no commas, so a bounded split is exact.
    let fields: Vec<&str> = line.splitn(COLUMNS, ',').collect();
    if fields.len() != COLUMNS {
        return Err(anyhow!("expected {COLUMNS} columns, got {}", fields.len()));
    }
    let flag = |field: &str| field == "t";
    let rating = |field: &str| field.parse::<f64>().ok();
    Ok(CorpusGame {
        nanoid: fields[0].to_string(),
        game_type: fields[1].to_string(),
        game_status: fields[2].to_string(),
        conclusion: fields[3].to_string(),
        rated: flag(fields[4]),
        white_rating: rating(fields[6]),
        black_rating: rating(fields[7]),
        white_bot: flag(fields[8]),
        black_bot: flag(fields[9]),
        tournament_queen_rule: flag(fields[10]),
        history: fields[13].trim().to_string(),
    })
}

/// Reads the export, gzipped or not, skipping the header.
pub fn read(path: &Path) -> Result<Vec<CorpusGame>> {
    let file = File::open(path).with_context(|| format!("opening {path:?}"))?;
    let reader: Box<dyn Read> = if path.extension().is_some_and(|ext| ext == "gz") {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut games = Vec::new();
    for (index, line) in BufReader::new(reader).lines().enumerate() {
        let line = line?;
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        games.push(parse_row(&line).with_context(|| format!("row {index}"))?);
    }
    Ok(games)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_export_row() {
        let game = parse_row(
            "1LJtkUXADQ,Base+MLP,Finished(1-0),Timeout,t,Bullet,1500,1500,f,t,t,29,2024-05-01 11:51:04.156401+02,wL ;bL wL-;wQ /wL;",
        )
        .expect("valid row");
        assert_eq!(game.nanoid, "1LJtkUXADQ");
        assert!(game.rated && game.black_bot && !game.white_bot);
        assert_eq!(game.moves(), vec!["wL ", "bL wL-", "wQ /wL"]);
    }
}
