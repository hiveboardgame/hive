use crate::{
    components::molecules::panel::Panel,
    responses::{GameResponse, TournamentResponse},
};
use leptos::{either::Either, html, prelude::*};
use leptos_use::use_element_size;
use shared_types::{TournamentGameResult, TournamentMode};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// One matchup: two players and how many games each won.
///
/// A bracket matchup is a two-game match, replayed until it is decisive, so a
/// score of 1-1 means "still being settled" rather than a drawn result.
#[derive(Clone)]
struct Matchup {
    left: Uuid,
    right: Uuid,
    left_wins: u32,
    right_wins: u32,
    decided: bool,
}

impl Matchup {
    fn winner(&self) -> Option<Uuid> {
        if !self.decided {
            return None;
        }
        if self.left_wins > self.right_wins {
            Some(self.left)
        } else {
            Some(self.right)
        }
    }

    fn involves(&self, player: Uuid) -> bool {
        self.left == player || self.right == player
    }

    fn same_pair(&self, other: &Self) -> bool {
        self.left == other.left && self.right == other.right
    }
}

fn unordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Groups a tournament's games into rounds of matchups.
///
/// Derived from the games rather than from `Standings`, which only carries
/// finishing tiers — the pairing structure a bracket diagram needs lives in
/// `round` plus the two player ids.
fn rounds_of(games: &[GameResponse]) -> Vec<(i32, Vec<Matchup>)> {
    let mut rounds: BTreeMap<i32, BTreeMap<(Uuid, Uuid), Matchup>> = BTreeMap::new();

    for game in games {
        let Some(round) = game.round else {
            continue;
        };
        let left = game.white_player.uid;
        let right = game.black_player.uid;
        let pair = unordered(left, right);
        let matchup = rounds
            .entry(round)
            .or_default()
            .entry(pair)
            .or_insert_with(|| Matchup {
                left: pair.0,
                right: pair.1,
                left_wins: 0,
                right_wins: 0,
                decided: false,
            });

        let winner = match game.tournament_game_result {
            TournamentGameResult::Winner(hive_lib::Color::White) => Some(left),
            TournamentGameResult::Winner(hive_lib::Color::Black) => Some(right),
            _ => None,
        };
        match winner {
            Some(winner) if winner == matchup.left => matchup.left_wins += 1,
            Some(_) => matchup.right_wins += 1,
            None => {}
        }
        if game.finished {
            matchup.decided = matchup.left_wins != matchup.right_wins;
        }
    }

    rounds
        .into_iter()
        .map(|(round, matchups)| (round, matchups.into_values().collect()))
        .collect()
}

/// Identifies a matchup across the whole diagram.
///
/// Keyed by round as well as by the pair, because the same two players can meet
/// again — a replayed bracket match, or a lower-bracket rematch.
fn matchup_key(round: i32, matchup: &Matchup) -> String {
    format!("{round}:{}:{}", matchup.left, matchup.right)
}

/// One line to draw: out of a matchup, from a named player's row, into another.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: String,
    /// Which side of the source box the line leaves from — a bracket line is only
    /// legible if it starts at the player it is about.
    pub from_left_row: bool,
    pub to: String,
    /// Whether this is the winner advancing or the loser dropping. A loser edge
    /// is the third-place match, or the fall into the lower bracket.
    pub winner: bool,
}

/// Works out every line in the diagram from who actually went where.
///
/// Derived from results rather than from layout adjacency, which is what the
/// earlier CSS-only version did. Adjacency cannot express a fork — the semifinals
/// feed both the final and the third-place match — nor a line that starts at one
/// player's row rather than the middle of a box, nor the upper-to-lower bracket
/// drops that make up half of a double elimination.
///
/// A player's next match is the *earliest* later one they appear in, so a run
/// through several rounds produces one edge per step rather than a fan.
fn edges_of(rounds: &[(i32, Vec<Entry>)]) -> Vec<Edge> {
    let mut edges = Vec::new();

    for (index, (round, entries)) in rounds.iter().enumerate() {
        for entry in entries {
            let Entry::Match(matchup) = entry else {
                // A bye has no loser and its player always goes through.
                if let Entry::Bye(player) = entry {
                    if let Some(to) = next_appearance(rounds, index, *player) {
                        edges.push(Edge {
                            from: entry.key(*round),
                            from_left_row: true,
                            to,
                            winner: true,
                        });
                    }
                }
                continue;
            };
            let Some(winner) = matchup.winner() else {
                continue;
            };
            let loser = if winner == matchup.left {
                matchup.right
            } else {
                matchup.left
            };
            let from = matchup_key(*round, matchup);

            for (player, is_winner) in [(winner, true), (loser, false)] {
                if let Some(to) = next_appearance(rounds, index, player) {
                    edges.push(Edge {
                        from: from.clone(),
                        from_left_row: player == matchup.left,
                        to,
                        winner: is_winner,
                    });
                }
            }
        }
    }
    edges
}

/// Where a player turns up next, which is the far end of an edge.
fn next_appearance(rounds: &[(i32, Vec<Entry>)], after: usize, player: Uuid) -> Option<String> {
    rounds.iter().skip(after + 1).find_map(|(round, entries)| {
        entries
            .iter()
            .find(|entry| entry.involves(player))
            .map(|entry| entry.key(*round))
    })
}

/// One slot in a column: a match, or somebody sitting the round out.
///
/// Byes are entries rather than a list appended to the column, so they can be
/// ordered with the matches. A bye's only meaning is where its player enters the
/// next round, and collected at the bottom it says nothing at all.
#[derive(Clone)]
enum Entry {
    Match(Box<Matchup>),
    Bye(Uuid),
}

impl Entry {
    /// Who goes through, which is what places this entry against the next round.
    fn advancing(&self) -> Option<Uuid> {
        match self {
            Self::Match(matchup) => matchup.winner(),
            Self::Bye(player) => Some(*player),
        }
    }

    fn is(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Match(a), Self::Match(b)) => a.same_pair(b),
            (Self::Bye(a), Self::Bye(b)) => a == b,
            _ => false,
        }
    }

    fn involves(&self, player: Uuid) -> bool {
        match self {
            Self::Match(matchup) => matchup.involves(player),
            Self::Bye(byed) => *byed == player,
        }
    }

    /// Identifies the entry in the DOM so the connector layer can measure it. A
    /// bye needs one too: a line runs from it to the match its player enters.
    fn key(&self, round: i32) -> String {
        match self {
            Self::Match(matchup) => matchup_key(round, matchup),
            Self::Bye(player) => format!("{round}:bye:{player}"),
        }
    }
}

/// Interleaves each round's byes with its matches, then orders the lot so every
/// entry sits against whatever it feeds in the next round.
fn entries_of(rounds: &[(i32, Vec<Matchup>)], infer_byes: bool) -> Vec<(i32, Vec<Entry>)> {
    let byes = if infer_byes {
        byes_of(rounds)
    } else {
        Vec::new()
    };

    let mut built: Vec<(i32, Vec<Entry>)> = rounds
        .iter()
        .enumerate()
        .map(|(index, (round, matchups))| {
            let mut entries: Vec<Entry> = matchups
                .iter()
                .map(|matchup| Entry::Match(Box::new(matchup.clone())))
                .collect();
            if let Some(sat_out) = byes.get(index) {
                entries.extend(sat_out.iter().copied().map(Entry::Bye));
            }
            (*round, entries)
        })
        .collect();

    for index in (1..built.len()).rev() {
        let successors = built[index].1.clone();
        let feeders = &mut built[index - 1].1;

        let mut ordered: Vec<Entry> = Vec::with_capacity(feeders.len());
        for successor in &successors {
            for feeder in feeders.iter() {
                let feeds = feeder
                    .advancing()
                    .is_some_and(|player| successor.involves(player));
                if feeds && !ordered.iter().any(|kept| kept.is(feeder)) {
                    ordered.push(feeder.clone());
                }
            }
        }
        // Anything with no derivable successor — undecided, or the third-place
        // match — keeps its order rather than being dropped.
        for feeder in feeders.iter() {
            if !ordered.iter().any(|kept| kept.is(feeder)) {
                ordered.push(feeder.clone());
            }
        }
        *feeders = ordered;
    }
    built
}

/// Who sat each round out.
///
/// Only sound for single elimination, where the field only ever shrinks. A
/// double elimination feeds its lower bracket from above mid-way through, so a
/// gap there means a player dropped, not that they rested — see `infer_byes`.
///
/// Elimination records no `tournament_byes` rows the way Swiss does, so these are
/// derived: in a knockout, anybody playing in the next round either won this one
/// or was never paired in it. A field that is not a power of two produces one of
/// these in every early round, and without them a player simply materialises out
/// of nowhere partway across the diagram.
fn byes_of(rounds: &[(i32, Vec<Matchup>)]) -> Vec<Vec<Uuid>> {
    let played_in = |matchups: &[Matchup]| -> Vec<Uuid> {
        matchups
            .iter()
            .flat_map(|matchup| [matchup.left, matchup.right])
            .collect()
    };

    rounds
        .iter()
        .enumerate()
        .map(|(index, (_, matchups))| {
            let here = played_in(matchups);
            let Some((_, next)) = rounds.get(index + 1) else {
                return Vec::new();
            };
            let mut sat_out: Vec<Uuid> = played_in(next)
                .into_iter()
                .filter(|player| !here.contains(player))
                .collect();
            sat_out.dedup();
            sat_out
        })
        .collect()
}

/// Re-orders each round so a matchup sits next to the one it shares a successor
/// with.
///
/// Rounds arrive keyed by player-id pair, so their order is effectively random.
/// That is invisible while each column is just a list, but it means nothing lines
/// up: the two quarter-finals that feed a semi-final can be at opposite ends of
/// their column. Working backwards from the final and placing each matchup under
/// the one it feeds puts pairs adjacent, which is what lets connectors be drawn.
///
/// A matchup feeds another when its winner plays in it. Undecided matchups have no
/// winner and therefore no derivable successor, so they keep their existing order
/// at the end of the round — as does a third-place match, which feeds nothing.
fn order_as_tree(rounds: &mut [(i32, Vec<Matchup>)]) {
    for index in (1..rounds.len()).rev() {
        let successors: Vec<Matchup> = rounds[index].1.clone();
        let feeders = &mut rounds[index - 1].1;

        let mut ordered: Vec<Matchup> = Vec::with_capacity(feeders.len());
        for successor in &successors {
            for feeder in feeders.iter() {
                let feeds = feeder
                    .winner()
                    .is_some_and(|winner| successor.involves(winner));
                if feeds && !ordered.iter().any(|kept| kept.same_pair(feeder)) {
                    ordered.push(feeder.clone());
                }
            }
        }
        // Whatever could not be placed — undecided, or a third-place match —
        // keeps its order rather than being dropped.
        for feeder in feeders.iter() {
            if !ordered.iter().any(|kept| kept.same_pair(feeder)) {
                ordered.push(feeder.clone());
            }
        }
        *feeders = ordered;
    }
}

/// Rounds counted from the end, the way brackets are actually named.
fn round_name(index: usize, total: usize) -> String {
    match total.saturating_sub(index) {
        1 => String::from("Final"),
        2 => String::from("Semifinals"),
        3 => String::from("Quarterfinals"),
        _ => format!("Round {}", index + 1),
    }
}

/// Splits the last round into the final and the third-place match.
///
/// Both land in the same `games.round`, so they arrive as one column of two
/// matchups labelled "Final" — which says nothing about which one decided the
/// title. They are told apart by who is in them: the final is contested by the
/// players who *won* their semifinals, the third-place match by the two who lost.
fn split_final(last: &[Matchup], semifinals: &[Matchup]) -> (Option<Matchup>, Option<Matchup>) {
    if last.len() < 2 || semifinals.is_empty() {
        return (last.first().cloned(), None);
    }

    let survivors: Vec<Uuid> = semifinals.iter().filter_map(Matchup::winner).collect();
    // Undecided semifinals leave nothing to reason from, so the column is left
    // as it is rather than guessing.
    if survivors.is_empty() {
        return (last.first().cloned(), last.get(1).cloned());
    }

    let decider = last
        .iter()
        .find(|matchup| survivors.iter().all(|player| matchup.involves(*player)));

    match decider {
        Some(decider) => {
            let bronze = last
                .iter()
                .find(|matchup| !std::ptr::eq(*matchup, decider))
                .cloned();
            (Some(decider.clone()), bronze)
        }
        None => (last.first().cloned(), last.get(1).cloned()),
    }
}

/// Which half of a double-elimination bracket a matchup belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Side {
    /// Nobody in it has lost yet.
    Upper,
    /// Everybody in it has already lost once.
    Lower,
    /// An unbeaten player against one who came back through the lower bracket.
    Decider,
}

/// Sorts every matchup into the upper or lower bracket.
///
/// This is derivable from the games after all, which an earlier version of this
/// file wrongly claimed needed a new column in the database: a double-elimination
/// match is a lower-bracket match exactly when both its players have already lost
/// one, so counting losses round by round recovers the structure. Without it both
/// halves are flattened into one row of columns, which is why an 8-player double
/// elimination rendered as six meaningless rounds.
fn sides_of(rounds: &[(i32, Vec<Matchup>)]) -> Vec<Vec<Side>> {
    let mut losses: HashMap<Uuid, usize> = HashMap::new();
    let mut sides = Vec::with_capacity(rounds.len());

    for (_, matchups) in rounds {
        // Classified against the standings *before* this round is applied, since
        // a match is defined by what its players had lost on entering it.
        let round_sides = matchups
            .iter()
            .map(|matchup| {
                let left = losses.get(&matchup.left).copied().unwrap_or(0);
                let right = losses.get(&matchup.right).copied().unwrap_or(0);
                match (left, right) {
                    (0, 0) => Side::Upper,
                    (0, _) | (_, 0) => Side::Decider,
                    _ => Side::Lower,
                }
            })
            .collect::<Vec<_>>();

        for matchup in matchups {
            if let Some(winner) = matchup.winner() {
                let loser = if winner == matchup.left {
                    matchup.right
                } else {
                    matchup.left
                };
                *losses.entry(loser).or_insert(0) += 1;
            }
        }
        sides.push(round_sides);
    }
    sides
}

/// A column of the diagram.
struct Column {
    heading: String,
    /// The `games.round` these matchups came from, which the matchup keys and
    /// therefore the connector lookups are built on.
    round: i32,
    entries: Vec<Entry>,
    placement: Placement,
}

/// Where a column's matchups sit vertically.
///
/// The final and the third-place match share a round, so centring both left them
/// level — which read as one line passing *through* the bronze match on its way to
/// the final, and gave no sense that one of them decided the tournament.
#[derive(Clone, Copy, PartialEq)]
enum Placement {
    /// Centred against whatever feeds it, which is what a round wants.
    Even,
    /// Raised, for the grand final: it ends the tournament and should not sit
    /// level with the lower bracket that feeds it.
    High,
}

impl Placement {
    /// Flex weights for the space above and below the matchups.
    ///
    /// The two are pushed well apart rather than nudged: the line from the lower
    /// semifinal to the final has to pass between them, and a small gap left it
    /// grazing the third-place box.
    fn spacers(self) -> (i32, i32) {
        match self {
            Self::Even => (0, 0),
            Self::High => (1, 9),
        }
    }
}

/// The columns to draw, with the third-place match split out into its own.
///
/// It shares a round with the final, and leaving them in one column gave that
/// column two headings — which pushed everything in it out of line with the round
/// before, since alignment depends on each column having the same amount of
/// non-matchup content above the boxes.
///
/// Third place comes *before* the final so the diagram ends on the match that
/// decided the title, which is what a reader is looking for.
/// Splits the rounds into the upper and lower halves of a double-elimination
/// bracket, or returns `None` when there is only one bracket to draw.
///
/// Gated on the mode rather than inferred from the losses. A single elimination
/// with a third-place match looks identical to loss-counting — both bronze players
/// have lost once — so inference labelled that match a "lower bracket final".
fn halves(
    rounds: &[(i32, Vec<Matchup>)],
    double: bool,
) -> Option<(Vec<(i32, Vec<Matchup>)>, Vec<(i32, Vec<Matchup>)>)> {
    if !double {
        return None;
    }
    let sides = sides_of(rounds);
    let has_lower = sides
        .iter()
        .any(|round| round.iter().any(|side| *side == Side::Lower));
    if !has_lower {
        return None;
    }

    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for ((round, matchups), round_sides) in rounds.iter().zip(&sides) {
        let take = |wanted: &[Side]| -> Vec<Matchup> {
            matchups
                .iter()
                .zip(round_sides)
                .filter(|(_, side)| wanted.contains(side))
                .map(|(matchup, _)| matchup.clone())
                .collect()
        };
        // The decider belongs at the end of the upper bracket, which is where it
        // is played from.
        let up = take(&[Side::Upper, Side::Decider]);
        let down = take(&[Side::Lower]);
        if !up.is_empty() {
            upper.push((*round, up));
        }
        if !down.is_empty() {
            lower.push((*round, down));
        }
    }

    // No promotion here: `sides_of` already routes the grand final to the upper
    // strip, because it is the one match with an unbeaten player in it. Moving
    // the last lower column up as well stole the lower-bracket final and left the
    // real grand final labelled as an upper-bracket round.
    Some((upper, lower))
}

fn columns_of(rounds: &[(i32, Vec<Matchup>)], infer_byes: bool) -> Vec<Column> {
    let total = rounds.len();
    let by_round = entries_of(rounds, infer_byes);
    let mut columns = Vec::with_capacity(total + 1);

    for (index, (round, matchups)) in rounds.iter().enumerate() {
        let is_last = index + 1 == total && total > 1;
        if !is_last {
            columns.push(Column {
                heading: round_name(index, total),
                round: *round,
                entries: by_round[index].1.clone(),
                placement: Placement::Even,
            });
            continue;
        }

        let (decider, bronze) = split_final(matchups, &rounds[index - 1].1);
        if let Some(bronze) = bronze {
            columns.push(Column {
                heading: String::from("Third place"),
                round: *round,
                // Centred, not pushed down: it sits where the two semifinal
                // losers naturally converge, one column before the final.
                entries: vec![Entry::Match(Box::new(bronze))],
                placement: Placement::Even,
            });
        }
        if let Some(decider) = decider {
            columns.push(Column {
                heading: round_name(index, total),
                round: *round,
                entries: vec![Entry::Match(Box::new(decider))],
                placement: Placement::Even,
            });
        }
    }
    columns
}

#[component]
pub fn Bracket(tournament: Signal<TournamentResponse>) -> impl IntoView {
    // Only a real double elimination has two brackets. Inferring it from loss
    // counts caught single elimination too, because a third-place match is also
    // contested by two players with one loss each.
    let double_elimination = tournament.with_untracked(|t| {
        matches!(
            t.mode.parse::<TournamentMode>(),
            Ok(TournamentMode::DoubleElimination)
        )
    });
    // Shared across every box so hovering a name follows that player through the
    // whole bracket, which is how you read a run.
    let highlighted = RwSignal::new(None::<Uuid>);
    // Every measured coordinate is relative to this, so it has to enclose both
    // halves of a double elimination.
    let container = NodeRef::<html::Div>::new();
    let rounds = move || {
        let mut rounds = tournament.with(|t| rounds_of(&t.games));
        order_as_tree(&mut rounds);
        rounds
    };
    let name_of = move |player: Uuid| {
        tournament.with(|t| {
            t.players
                .get(&player)
                .map(|user| user.username.clone())
                .unwrap_or_else(|| String::from("—"))
        })
    };

    // Named separately from the column layout because the champion is the one
    // thing somebody opening a finished bracket is actually looking for.
    let champion = move || {
        let rounds = rounds();
        let (last, semifinals) = (rounds.last()?, rounds.len().checked_sub(2)?);
        let (decider, _) = split_final(&last.1, &rounds[semifinals].1);
        decider?.winner().map(name_of)
    };

    view! {
        <Panel title="Results" class="min-w-0">
            <Show when=move || champion().is_some()>
                <p class="flex gap-2 items-center mb-3 text-sm font-bold text-gray-900 dark:text-gray-100">
                    <span aria-hidden="true">"🏆"</span>
                    <span>
                        {move || champion().map(|name| format!("{name} wins the tournament"))}
                    </span>
                </p>
            </Show>
            // Tagged so a browser test can assert the bracket scrolls *here*
            // rather than pushing the page sideways — the two are
            // indistinguishable from a page-level overflow check alone.
            <div class="overflow-x-auto" data-testid="bracket-scroll">
                {move || {
                    let rounds = rounds();
                    let edges = edges_of(&entries_of(&rounds, !double_elimination));
                    match halves(&rounds, double_elimination) {
                        Some((upper, lower)) => {
                            Either::Left(
                                // One overlay across both halves. Each strip used to draw its
                                // own, which meant an upper-bracket drop into the lower bracket
                                // was measured against the wrong container and landed nowhere
                                // near its target.
                                // Double elimination is two brackets, and stacking them is the
                                // only way either reads: interleaved in one row, an 8-player
                                // event looked like six unrelated rounds.
                                view! {
                                    <div node_ref=container class="relative space-y-12">
                                        <Connectors edges container />
                                        <Strip
                                            rounds=upper
                                            label="Upper"
                                            grand_final=true
                                            highlighted
                                            name_of=name_of.clone()
                                        />
                                        // Shifted one column right so every drop from
                                        // the bracket above runs down and forward.
                                        <Strip
                                            rounds=lower
                                            label="Lower"
                                            highlighted
                                            name_of=name_of.clone()
                                            indent_columns=1
                                        />
                                    </div>
                                },
                            )
                        }
                        None => {
                            Either::Right(
                                view! {
                                    <div node_ref=container class="relative">
                                        <Connectors edges container />
                                        <Strip
                                            rounds
                                            highlighted
                                            name_of=name_of.clone()
                                            infer_byes=true
                                        />
                                    </div>
                                },
                            )
                        }
                    }
                }}
            </div>
        </Panel>
    }
}

/// One horizontal run of rounds, with the connectors between them.
#[component]
fn Strip(
    rounds: Vec<(i32, Vec<Matchup>)>,
    highlighted: RwSignal<Option<Uuid>>,
    name_of: impl Fn(Uuid) -> String + Clone + 'static,
    /// Names the half, for a double-elimination bracket.
    #[prop(optional)]
    label: Option<&'static str>,
    /// Indents the strip by whole columns. Shifting the lower bracket right lets
    /// each drop from the upper bracket run down and forward, instead of doubling
    /// back on itself.
    #[prop(optional)]
    indent_columns: usize,
    /// The upper strip of a double elimination ends in the grand final, which is
    /// its own thing rather than that bracket's final.
    #[prop(optional)]
    grand_final: bool,
    /// Whether a gap in a player's record means they sat the round out.
    ///
    /// Only true for single elimination. In a double elimination a player who
    /// loses above drops into the lower bracket mid-way, so they turn up in a
    /// round without having played the one before it — which is a drop, not a bye,
    /// and labelling it one put "BYE" against three players who had just lost.
    #[prop(optional)]
    infer_byes: bool,
) -> impl IntoView {
    let mut columns = columns_of(&rounds, infer_byes);
    // A double elimination's halves are not knockout rounds counting down to a
    // final, so naming them "Quarterfinals" by position is simply wrong. Each half
    // is numbered, and only the matches that end it get a name.
    if let Some(half) = label {
        let last = columns.len().saturating_sub(1);
        for (index, column) in columns.iter_mut().enumerate() {
            column.heading = if index == last && grand_final {
                String::from("Grand final")
            } else if index == last {
                format!("{half} final")
            } else {
                format!("Round {}", index + 1)
            };
        }
    }
    if grand_final {
        if let Some(last) = columns.last_mut() {
            last.placement = Placement::High;
        }
    }
    // Must match `.ui-bracket-column`'s `w-52` plus the strip's `gap-16`, so an
    // indented half lands exactly one step along the grid the other half uses.
    let offset = (indent_columns > 0)
        .then(|| format!("margin-left: calc({indent_columns} * (13rem + 4rem))"))
        .unwrap_or_default();

    view! {
        {label
            .map(|label| {
                view! {
                    <p class="mb-1 text-xs font-bold tracking-tight text-gray-500 uppercase dark:text-gray-400">
                        {format!("{label} bracket")}
                    </p>
                }
            })}
        // The gap is what the connectors run through. The columns used to be
        // separated by dedicated line-drawing columns; without them, and without
        // this, every round sat flush against the next and each line had zero
        // horizontal distance to cover.
        <div class="flex gap-16 items-stretch min-w-fit" style=offset>
            {columns
                .into_iter()
                .map(|column| {
                    let name_of = name_of.clone();
                    let (above, below) = column.placement.spacers();
                    view! {
                        <div class="ui-bracket-column">
                            // Equal-height slots still do the vertical placement —
                            // a column of four slots centres each box against the
                            // two feeding it. The lines themselves are measured and
                            // drawn over the top.
                            <div class="flex flex-col flex-1">
                                // Weighted spacers rather than a fixed offset, so
                                // the final rides above the third-place match by a
                                // share of the column however tall the bracket is.
                                {(above > 0)
                                    .then(|| {
                                        view! { <div style=format!("flex: {above} 1 0%")></div> }
                                    })} // Inside the spacers, so a heading sits directly
                                // over its own matches. Pinned to the top of the
                                // column it floated far above the final and the
                                // third-place match, naming nothing in particular.
                                {column
                                    .entries
                                    .iter()
                                    .enumerate()
                                    .map(|(index, entry)| {
                                        let heading = (index == 0).then(|| column.heading.clone());
                                        let key = entry.key(column.round);
                                        let slot = match entry {
                                            Entry::Match(matchup) => {
                                                Either::Left(
                                                    // The heading rides inside the first slot
                                                    // rather than at the top of the column: a
                                                    // box is centred in its slot, so a pinned
                                                    // heading drifted further from its own
                                                    // matches with every round.
                                                    view! {
                                                        <MatchupBox
                                                            matchup=(**matchup).clone()
                                                            highlighted
                                                            name_of=name_of.clone()
                                                            key=key
                                                        />
                                                    },
                                                )
                                            }
                                            Entry::Bye(player) => {
                                                let player = *player;
                                                let name = name_of(player);
                                                Either::Right(
                                                    view! {
                                                        // Sized and keyed like a matchup so it
                                                        // takes a slot in the tree and a line can
                                                        // run from it into the match it feeds.
                                                        <div
                                                            class="flex gap-2 justify-between py-1 px-2 w-full text-xs rounded border border-dashed border-pillbug-teal/60 text-pillbug-teal"
                                                            title="No opponent this round — advanced automatically"
                                                            data-matchup=key
                                                            on:mouseenter=move |_| highlighted.set(Some(player))
                                                        >
                                                            <span class="truncate" data-row="left">
                                                                {name}
                                                            </span>
                                                            <span class="tracking-tight uppercase text-[10px]">
                                                                "bye"
                                                            </span>
                                                        </div>
                                                    },
                                                )
                                            }
                                        };
                                        view! {
                                            <div class="flex flex-col justify-center ui-bracket-feed">
                                                {heading
                                                    .map(|heading| {
                                                        view! { <h3 class=SUBHEADING>{heading}</h3> }
                                                    })} {slot}
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                                {(below > 0)
                                    .then(|| {
                                        view! { <div style=format!("flex: {below} 1 0%")></div> }
                                    })}
                            </div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Draws the edges as an SVG overlay, measured from where the boxes actually are.
///
/// Measurement rather than CSS because the lines have to start at one player's row
/// and can fork — the semifinal losers go to the third-place match while the
/// winners go to the final — and neither is expressible with borders on sibling
/// elements. Redrawn whenever the container resizes, since every coordinate here
/// is a laid-out pixel.
#[component]
fn Connectors(edges: Vec<Edge>, container: NodeRef<html::Div>) -> impl IntoView {
    let paths = RwSignal::new(Vec::<(String, bool)>::new());
    let size = use_element_size(container);

    Effect::new(move |_| {
        // Subscribing to the size is what re-runs this after a resize or a
        // late-arriving font.
        let (width, height) = (size.width.get(), size.height.get());
        let Some(root) = container.get() else {
            return;
        };
        let origin = root.get_bounding_client_rect();

        // Where a row's outgoing point and a box's incoming point sit, relative
        // to the container.
        let anchor = |key: &str, row: Option<&str>| -> Option<(f64, f64, f64)> {
            let document = web_sys::window()?.document()?;
            let selector = format!("[data-matchup=\"{key}\"]");
            let box_element = document.query_selector(&selector).ok()??;
            let target = match row {
                Some(row) => box_element
                    .query_selector(&format!("[data-row=\"{row}\"]"))
                    .ok()??,
                None => box_element.clone(),
            };
            let rect = target.get_bounding_client_rect();
            let outer = box_element.get_bounding_client_rect();
            Some((
                outer.left() - origin.left(),
                outer.right() - origin.left(),
                rect.top() + rect.height() / 2.0 - origin.top(),
            ))
        };

        let mut drawn = Vec::new();
        for edge in &edges {
            let row = if edge.from_left_row { "left" } else { "right" };
            let (Some((_, from_right, from_y)), Some((to_left, _, to_y))) =
                (anchor(&edge.from, Some(row)), anchor(&edge.to, None))
            else {
                continue;
            };
            // An elbow rather than a diagonal: brackets are read as orthogonal
            // routing, and a straight line between distant rounds crosses boxes.
            //
            // The turn happens just before the destination rather than halfway
            // across. Halfway put every vertical segment at the same x, so
            // unrelated lines crossing the same gap — which is most of a double
            // elimination — stacked on top of one another. Turning late keeps each
            // line on its own row until the last moment, while the two feeding one
            // box still converge on the same column.
            // Lanes, keyed on the destination's height. Now that both halves share
            // one column grid, every line into a given column would otherwise turn
            // at the same x and the verticals would stack; before the grid, the two
            // halves drifting apart hid that by accident. Edges into the *same* box
            // share a `to_y`, so they still converge as a bracket should.
            let lane = ((to_y / 29.0) as i64).rem_euclid(6) as f64;
            let mid = (to_left - 10.0 - lane * 9.0).max(from_right);
            drawn.push((
                format!("M {from_right} {from_y} H {mid} V {to_y} H {to_left}"),
                edge.winner,
            ));
        }
        let _ = (width, height);
        paths.set(drawn);
    });

    view! {
        <svg
            class="absolute inset-0 w-full h-full pointer-events-none"
            aria-hidden="true"
            fill="none"
        >
            {move || {
                paths
                    .get()
                    .into_iter()
                    .map(|(path, winner)| {
                        view! {
                            <path
                                d=path
                                // A loser's route is dashed, so a drop into the
                                // lower bracket or across to the third-place match
                                // is not mistaken for an advance.
                                stroke-dasharray=if winner { "" } else { "3 3" }
                                class=if winner {
                                    "stroke-gray-400 dark:stroke-gray-600"
                                } else {
                                    "stroke-gray-300 dark:stroke-gray-700"
                                }
                                stroke-width="1.5"
                            />
                        }
                    })
                    .collect_view()
            }}
        </svg>
    }
}

const SUBHEADING: &str =
    "text-xs font-bold tracking-tight text-gray-700 uppercase dark:text-gray-300";

#[component]
fn MatchupBox(
    matchup: Matchup,
    highlighted: RwSignal<Option<Uuid>>,
    name_of: impl Fn(Uuid) -> String + Clone + 'static,
    /// Identifies the box so the connector layer can find it in the DOM and
    /// measure where its two rows actually are.
    key: String,
) -> impl IntoView {
    // The winner is only emphasised once the match is actually settled — a 1-1
    // bracket match is replayed, not drawn.
    let left_won = matchup.decided && matchup.left_wins > matchup.right_wins;
    let right_won = matchup.decided && matchup.right_wins > matchup.left_wins;
    let (left, right) = (matchup.left, matchup.right);

    // Alternating shades per side, so a matchup reads as two players rather than
    // one block of four values — the same pattern the dense tables use.
    let side = move |player: Uuid, won: bool, odd: bool| {
        move || {
            let shade = if highlighted.get() == Some(player) {
                "bg-pillbug-teal/25"
            } else if odd {
                "bg-odd-light dark:bg-surface-row-odd"
            } else {
                "bg-even-light dark:bg-surface-row-even"
            };
            let weight = if won {
                "font-bold text-gray-900 dark:text-gray-100"
            } else {
                "text-gray-600 dark:text-gray-400"
            };
            format!("flex justify-between gap-2 px-2 py-1 {shade} {weight}")
        }
    };

    view! {
        <div
            class="overflow-hidden w-full text-sm rounded border border-gray-300 dark:border-gray-700"
            data-matchup=key
        >
            <div
                class=side(left, left_won, true)
                data-row="left"
                on:mouseenter=move |_| highlighted.set(Some(left))
            >
                <span class="truncate">{name_of(matchup.left)}</span>
                <span class="tabular-nums">{matchup.left_wins}</span>
            </div>
            <div
                class=side(right, right_won, false)
                data-row="right"
                on:mouseenter=move |_| highlighted.set(Some(right))
            >
                <span class="truncate">{name_of(matchup.right)}</span>
                <span class="tabular-nums">{matchup.right_wins}</span>
            </div>
        </div>
    }
}
