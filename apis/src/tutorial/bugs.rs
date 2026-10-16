use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "bugs",
    title: "The pieces",
    summary: "Eight pieces, eight ways to move. Learn each one by playing it.",
    lessons: &[
        QUEEN,
        FREEDOM_TO_MOVE,
        DONT_BREAK_THE_HIVE,
        ANT_FIRST_MOVES,
        ANT,
        ANT_ATTACK,
        ANT_PIN,
        SPIDER,
        SPIDER_DEAD_END,
        GRASSHOPPER,
        BEETLE,
        BEETLE_GATE,
        MOSQUITO,
        LADYBUG,
        PILLBUG,
    ],
};

const QUEEN: Lesson = Lesson {
    id: "bugs.queen",
    title: "Queen Bee",
    intro: &[
        "Your Queen [[wQ]] is your most important piece: if she gets surrounded, you lose the game. Think of her like the king in chess.",
        "She moves by **crawling** one hex per turn. Many pieces in Hive move by crawling.",
        "**Crawling** simply means sliding along the edge of the hive.",
    ],
    brief: "Supplied HOP, White to move. wQ steps to bA2\\, touching the black Ant; Black places bG1 at bL\\; wQ then reaches bA2-, the green hex in line with both black Ants, pinning them both.",
    setup: Setup::Position("A+q+M2-(l+La!-a2-Q),w"),
    steps: &[
        Step::play(
            "You play as White. Make your Queen [[wQ]] reach the green hex in 2 moves to complete the lesson. Click on your pieces to see where they can move.",
            Move {
                piece: "wQ",
                to: "bA2\\",
            },
            "Aim to reach the green hex as quickly as possible.",
        )
        .highlighted(&["bA2-"])
        .hint_after(2)
        .reply("bG1", "bL\\")
        .your_turn(),
        Step::play(
            "You play as White. Make your Queen [[wQ]] reach the green hex in 2 moves to complete the lesson. Click on your pieces to see where they can move.",
            Move {
                piece: "wQ",
                to: "bA2-",
            },
            "Aim to reach the green hex as quickly as possible.",
        )
        .highlighted(&["bA2-"])
        .hint_after(2),
        Step::read(
            "Well done! The Queen [[wQ]] has reached her destination and pinned the two Ants [[bA1]] [[bA2]] in the process.",
            &[],
        )
        .with_completion_tip(
            "Tip: The Queen [[wQ]] doesn't need to move often, but take your chances to escape from your opponent's pieces if you can!",
        ),
    ],
    outro: "",
};

const FREEDOM_TO_MOVE: Lesson = Lesson {
    id: "bugs.freedom_to_move",
    title: "Freedom to Move Rule",
    intro: &[
        "Crawling is only possible if pieces can slide along the edges of the hive. Here, the White Ladybug [[wL]] and Ant [[wA2]] obstruct the movement of the Queen [[wQ]] according to the Freedom to Move Rule.",
        "These formations are also called Gates (you will learn more about them later!).",
    ],
    brief: "Supplied HOP, White to move. The hex -wL is gated by wL and wA2. Move wA2 anywhere except -wL to open the Gate; Black places bG1 beside bA2 when legal, or elsewhere if White blocks every adjacent placement. Then wQ crawls into -wL.",
    setup: Setup::Position("AB-M+B1-(q-l+La+s!3-(Qa1+ab1-A)),w"),
    steps: &[
        Step::play(
            "You play as White. Move your Ant [[wA2]] to allow your Queen [[wQ]] to reach the green hex.",
            MoveExcept {
                piece: "wA2",
                except: "-wL",
            },
            "Aim to reach the green hex as quickly as possible.",
        )
        .highlighted(&["-wL"])
        .hint_after(2)
        .reply_placement_near("bG1", "bA2")
        .your_turn(),
        Step::play(
            "You play as White. Move your Queen [[wQ]] into the green hex.",
            Move {
                piece: "wQ",
                to: "-wL",
            },
            "Aim to reach the green hex as quickly as possible.",
        )
        .highlighted(&["-wL"])
        .hint_after(2),
        Step::read(
            "Well done! Your Ant [[wA2]] opened the Gate, allowing your Queen [[wQ]] to crawl through.",
            &[],
        ),
    ],
    outro: "",
};

const DONT_BREAK_THE_HIVE: Lesson = Lesson {
    id: "bugs.dont_break_the_hive",
    title: "Don't break the hive",
    intro: &[
        "You are not allowed to break the hive during a move, even if it would reconnect at the end of the move.",
        "Only your White Queen [[wQ]] can move here. The blank White and Black tiles stay fixed and form the hive.",
        "Keep your Queen [[wQ]] touching the hive as she crawls around it. Reach the green hex to completely surround the Black Queen [[bQ]].",
    ],
    brief: "Supplied HOP with fixed blank tiles for every non-Queen piece. Only wQ may move. Crawl around the hive to bQ/, the last hole beside bQ. No opponent moves. The adjacent wM/ hex is red because wQ would lose contact while sliding there directly.",
    setup: Setup::BlankPosition("A+B-aa1-(lqA1-L+A-a2+M)3+m!3-Q,w"),
    steps: &[
        Step::play(
            "You play as White. Move your Queen [[wQ]] around the hive into the green hex to surround the Black Queen [[bQ]].",
            Travel { piece: "wQ", to: "bQ/" },
            "Keep your Queen touching the hive and aim for the green hex.",
        )
        .highlighted(&["bQ/"])
        .forbidden(&["wM/"])
        .hint_after(2),
        Step::read("Well done!", &[]).forbidden(&["wM/"]),
    ],
    outro: "",
};

const ANT_FIRST_MOVES: Lesson = Lesson {
    id: "bugs.ant_first_moves",
    title: "Ant (first moves)",
    intro: &[
        "The Ant [[wA1]] can crawl around the edge of the hive in a single turn.",
        "Move your Ant [[wA1]] next to the Black Queen [[bQ]] to begin an attack.",
    ],
    brief: "Supplied HOP ALl-qp!1+Q,w. Move the existing wA1 to any legal hex adjacent to bQ.",
    setup: Setup::Position("ALl-qp!1+Q,w"),
    steps: &[
        Step::play(
            "You play as White. Move your Ant [[wA1]] to any space next to the Black Queen [[bQ]].",
            MovePiece { piece: "wA1", touching: Some("bQ") },
            "Move your Ant next to the Black Queen.",
        )
        .hint_after(2),
        Step::read("Well done! Your Ant [[wA1]] is attacking the Black Queen [[bQ]].", &[]),
    ],
    outro: "",
};

const ANT: Lesson = Lesson {
    id: "bugs.ant",
    title: "Ant",
    intro: &[
        "The Ant [[wA1]] crawls around the edge of the hive, moving as many steps as you like.",
        "According to the Freedom to Move Rule, the Ant [[wA1]] cannot slide through narrow entrances (red hex).",
        "Each player starts the game with 3 Ants [[wA1]] in reserve.",
    ],
    brief: "From a 2000+ game (corpus game 4, after four moves each), White to move. The hole -bP between the Queens is gated and red. wA1 must move to either bQ- or bQ/, the only reachable hexes adjacent to bQ. Show those two green destinations only after two incorrect attempts.",
    setup: Setup::Position("A+P+QL-lb5-q-p,w"),
    steps: &[
        Step::play(
            "You play as White. Play any move with your Ant [[wA1]] to attack the opponent's Queen [[bQ]].",
            MovePiece {
                piece: "wA1",
                touching: Some("bQ"),
            },
            "Move your Ant next to the Black Queen.",
        )
        .forbidden(&["-bP"])
        .highlight_hint_after(2, &["bQ-", "bQ/"]),
        Step::read(
            "Well done! Feel free to restart the lesson and try to move the Ant [[wA1]] differently.",
            &[],
        )
        .forbidden(&["-bP"])
        .with_completion_tip(
            "Tip: The Ant [[wA1]] can reach almost any space immediately. It's the most flexible piece you have!",
        ),
    ],
    outro: "",
};

const ANT_ATTACK: Lesson = Lesson {
    id: "bugs.ant_attack",
    title: "Ant (attack!)",
    intro: &["This lesson is being prepared."],
    brief: "Placeholder for a future Ant attack lesson; position and instructions to follow.",
    setup: Setup::Pending,
    steps: &[Step::read("This lesson is being prepared.", &[])],
    outro: "",
};

const ANT_PIN: Lesson = Lesson {
    id: "bugs.ant_pin",
    title: "Ant (pin!)",
    intro: &["This lesson is being prepared."],
    brief: "Placeholder for a future Ant pin lesson; position and instructions to follow.",
    setup: Setup::Pending,
    steps: &[Step::read("This lesson is being prepared.", &[])],
    outro: "",
};

const SPIDER: Lesson = Lesson {
    id: "bugs.spider",
    title: "Spider",
    intro: &["The Spider slides exactly three hexes around the hive: not two, not four. It can't step back onto a hex it just left during the same move."],
    brief: "From a 2000+ game (corpus game 9, after five moves each), White to move. Green hex for wS1: -wQ, exactly three slides away.",
    setup: Setup::Position("QA+L+S1+M2-m3-(l+a1-q),w"),
    steps: &[
        Step::read(
            "Click your Spider to see where it can go. Every hex it offers is exactly three steps along the edge of the hive: the hexes in between are out of bounds.",
            &[],
        ),
        Step::play(
            "Move your Spider to the green hex: exactly three steps.",
            Visit {
                piece: "wS1",
                hexes: &["-wQ"],
            },
            "Count three hexes along the edge of the hive, without turning back.",
        ),
    ],
    outro: "Spiders are limited, but that precision is useful. A Spider is perfect for slipping into one specific hex next to the enemy Queen.",
};

const SPIDER_DEAD_END: Lesson = Lesson {
    id: "bugs.spider_dead_end",
    title: "A Spider with nowhere to go",
    intro: &[
        "A Spider needs a complete path of three slides. If every route runs into a dead end before the third step, it can't move at all, even though nothing pins it.",
    ],
    brief: "From a 2000+ game (corpus game 33, after 13 moves each), White to move. wS1 is not pinned and has two open neighbours (-bA2, -bL), but no route ever reaches a third step, so it has no legal move.",
    setup: Setup::Position("A+al+bp3-(L+Q2=b)4+(qA+BmA1=M1+S+a),w"),
    steps: &[Step::read(
        "Click your Spider. It isn't pinned, and it has open hexes next to it, but every route runs into a dead end before the third step. So it has no moves at all.",
        &["-bA2", "-bL"],
    )],
    outro: "Trapping a Spider like this costs nothing extra: the pieces doing it are just standing there. Keep it in mind when you choose where to put yours.",
};

const GRASSHOPPER: Lesson = Lesson {
    id: "bugs.grasshopper",
    title: "Grasshopper",
    intro: &[
        "The Grasshopper jumps in a straight line over one or more pieces and lands in the first empty hex behind them. It can't jump over an empty hex.",
        "Because it jumps instead of sliding, gates mean nothing to it. It can drop into holes no walking piece can reach.",
    ],
    brief: "From a 2000+ game (corpus game 26, after 11 moves each), White to move. wG1 jumps into a walled-in hole (-bQ); Black's scripted reply bG1 bP\\; then wG1 jumps out again to -bA2. Highlight and arrow for each jump.",
    setup: Setup::Position("A+ap+ql+L+B-a5=B2+b4-M+G6-(Qs1-a),w"),
    steps: &[
        Step::play(
            "Jump your Grasshopper into the highlighted hole.",
            Move {
                piece: "wG1",
                to: "-bQ",
            },
            "The Grasshopper moves in straight lines only, over pieces, never over gaps.",
        )
        .highlighted(&["-bQ"])
        .with_arrows(&[("wG1", "-bQ")])
        .reply("bG1", "bP\\"),
        Step::play(
            "No walking piece could have got in there. Now jump out again, to the highlighted hex.",
            Move {
                piece: "wG1",
                to: "-bA2",
            },
            "The Grasshopper moves in straight lines only, over pieces, never over gaps.",
        )
        .highlighted(&["-bA2"])
        .with_arrows(&[("wG1", "-bA2")]),
    ],
    outro: "A Grasshopper is a sniper: it can fill a hex from far away that walking pieces can't get to at all.",
};

const BEETLE: Lesson = Lesson {
    id: "bugs.beetle",
    title: "Beetle",
    intro: &[
        "The Beetle moves one hex at a time, like the Queen, but it can also climb on top of the hive and walk along it.",
        "A piece with a Beetle on top can't move. And the stack counts as the colour of whatever is on top, which changes where new pieces may be placed.",
    ],
    brief: "From a 2000+ game (corpus game 22, after five moves each), White to move. wB1 climbs onto bG1; the stack then opens a new white placement next to it.",
    setup: Setup::Position("ABM1+(L+l+qp1-Q)2-g,w"),
    steps: &[
        Step::play(
            "Climb on top of the black Grasshopper.",
            Reach(OnTop("wB1", "bG1")),
            "Your Beetle moves one hex, and that hex may be on top of another piece.",
        )
        .reply("bA1", "\\bP"),
        Step::read(
            "The black Grasshopper underneath is stuck for as long as your Beetle stays. And the stack is now white, as far as placing goes. To see what is under a stack, right-click it, or press and hold it on a touch screen. Try it now.",
            &["wB1"],
        ),
        Step::play(
            "Place a Grasshopper right next to the stack, where the black Grasshopper used to block you.",
            Reach(Touching("wG", "bG1")),
            "Choose a hex touching your Beetle and no other black piece.",
        ),
    ],
    outro: "Beetles are slow, but climbing makes them some of the strongest pieces in the late game. Covering the enemy Queen is often how games are won.",
};

const BEETLE_GATE: Lesson = Lesson {
    id: "bugs.beetle_gate",
    title: "Beetle gates",
    intro: &["Gates exist on top of the hive too. A Beetle can't squeeze between two stacks that are higher than both where it starts and where it lands."],
    brief: "hivegame.com/game/OiLpk05USK1S (229b05d2, 1963 vs 1990), after move 40. wB1 sits on bQ, whose last empty kill spot is -bB1, but bB1 (on wG1) and bB2 (on wM) gate the way down. Puzzle: climb onto either black Beetle; Black answers bA1 -wA3 as in the game; then wB1 drops into -bB1 and wins. Both routes verified.",
    setup: Setup::Position("A+GA+L+G+plPQ+A+a2=b6=M6=b9=m1-a2+(q1=B)4-Ga9-g,w"),
    steps: &[
        Step::play(
            "Your Beetle on the black Queen can't step down into the highlighted hex: the two black Beetles beside the way down form a gate. Find a way around it.",
            Reach(Not(&All(&[
                Not(&OnTop("wB1", "bB1")),
                Not(&OnTop("wB1", "bB2")),
            ]))),
            "Climb your Beetle onto one of the two ringed black Beetles first.",
        )
        .marked(&["bB1", "bB2"])
        .highlighted(&["-bB1"])
        .reply("bA1", "-wA3"),
        Step::play(
            "From up there nothing blocks the way down. Win the game.",
            Move {
                piece: "wB1",
                to: "-bB1",
            },
            "Click your Beetle, then the highlighted hex.",
        )
        .highlighted(&["-bB1"]),
    ],
    outro: "The same gate on top of the hive can stop a Pillbug from lifting a piece over itself, and a Ladybug from walking across the top.",
};

const MOSQUITO: Lesson = Lesson {
    id: "bugs.mosquito",
    title: "Mosquito",
    intro: &[
        "The Mosquito has no move of its own. It copies the movement of any piece it touches. Next to an Ant it moves like an Ant, next to a Beetle it can climb.",
        "Touching only another Mosquito, it can't move at all. Once it has climbed on top of the hive, it moves like a Beetle until it comes down.",
    ],
    brief: "From a 2000+ game (corpus game 36, after four moves each), White to move. wM moves next to bQ (bQ-), where it touches a Beetle, then borrows the Beetle's climb onto bQ.",
    setup: Setup::Position("AM-Pmq1-Q4+b4-p,w"),
    steps: &[
        Step::play(
            "Move your Mosquito to the green hex, next to the black Queen. Click it first to see which moves it borrows from its neighbours.",
            Visit {
                piece: "wM",
                hexes: &["bQ-"],
            },
            "Your Mosquito can reach the green hex in one move.",
        )
        .reply("bA1", "-bP"),
        Step::play(
            "Now it touches a Beetle. Borrow the Beetle's climb and get on top of the black Queen.",
            Reach(OnTop("wM", "bQ")),
            "Use the Beetle's climb: one hex, up onto the Queen.",
        ),
    ],
    outro: "Put a Mosquito next to the right pieces and it can do almost anything. That's why many players rate it the strongest piece after the Queen.",
};

const LADYBUG: Lesson = Lesson {
    id: "bugs.ladybug",
    title: "Ladybug",
    intro: &[
        "The Ladybug always moves exactly three hexes: two steps on top of the hive, then one step down into an empty hex.",
        "It can't stop on top, and it can't skip the climb. It's quick and flexible over short distances and can land inside pockets walking pieces can't reach.",
    ],
    brief: "From a 2000+ game (corpus game 19, after seven moves each), White to move. Green hex for wL: bP-, a walled-in pocket.",
    setup: Setup::Position("A+al-L+Q+M+Ps+p+m-q,w"),
    steps: &[Step::play(
        "Move your Ladybug into the walled-in pocket: up, along the top, and down.",
        Visit {
            piece: "wL",
            hexes: &["bP-"],
        },
        "Two steps on top of the hive, then one step down.",
    )],
    outro: "No walking piece could ever get in there. Ladybugs are at their best in crowded positions.",
};

const PILLBUG: Lesson = Lesson {
    id: "bugs.pillbug",
    title: "Pillbug",
    intro: &[
        "The Pillbug walks one hex at a time, like the Queen. Instead of moving, it can pick up a piece next to it, yours or your opponent's, carry it over its back, and put it down in another empty hex next to it.",
        "Some limits apply. It can't lift a piece that moved on the last turn, a piece under a stack, or a piece whose removal would split the hive. And a piece that was just thrown can't do anything on its next turn.",
    ],
    brief: "From a 2000+ game (corpus game 150, after six moves each), White to move. wP throws bM to wP\\; then wP lifts the crowded wQ to wA2-, where she has at most two neighbours.",
    setup: Setup::Position("AL+la-p1-(MQm1+A-P)3-q,w"),
    steps: &[
        Step::play(
            "Use your Pillbug to lift the black Mosquito onto the green hex.",
            Visit {
                piece: "bM",
                hexes: &["wP\\"],
            },
            "Select the black Mosquito next to your Pillbug, then the green hex.",
        )
        .reply("bA2", "\\bP"),
        Step::read(
            "Black placed an Ant instead of moving the Mosquito: a piece that was just thrown is stunned and can't move on the next turn. It also works the other way: if Black moves a piece next to your Pillbug, you can't throw it straight away.",
            &[],
        ),
        Step::play(
            "Your Queen is getting crowded. Use the Pillbug to lift her somewhere with more room.",
            Reach(Not(&KillSpotsFilled {
                queen: Color::White,
                at_least: 3,
            })),
            "Select your Queen and drop her on an empty hex next to the Pillbug.",
        ),
    ],
    outro: "The Pillbug is the most complicated piece, and the best defender in the game. A Pillbug next to its own Queen can lift her out of almost any surround, which changes how attacks have to work. The tactics chapter is built around it.",
};
