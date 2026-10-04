use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "basics",
    title: "The basics",
    summary: "How a turn works, how pieces enter the game, and the two rules every move obeys.",
    lessons: &[
        WELCOME,
        SURROUND,
        PLACING,
        QUEEN_RULE,
        ONE_HIVE,
        FREEDOM_TO_MOVE,
        GATES_AND_DOORS,
    ],
};

const WELCOME: Lesson = Lesson {
    id: "basics.welcome",
    title: "Welcome to Hive",
    intro: &["Hive has no board: the pieces are the board. You win by surrounding your opponent's Queen Bee."],
    brief: "Empty board, White to move. The faint hex grid is shown while placing.",
    setup: Setup::Position(",w"),
    steps: &[Step::play(
        "White starts. Place any piece from your reserve, except the Queen.",
        Reach(All(&[AnyPlaced(Color::White), Not(&Placed("wQ"))])),
        "Click a piece in your reserve, then the highlighted hex. The Queen may not open the game.",
    )
    .with_grid()],
    outro: "Each turn you do one thing: place a new piece or move one already in play. Pieces are never captured.",
};

const SURROUND: Lesson = Lesson {
    id: "basics.surround",
    title: "Surround the Queen",
    intro: &["This is the end of a real game between two strong players. The black Queen has pieces on five of her six sides, and it's White's move."],
    brief: "From a 2000+ game (corpus game 420, after 14 moves each), White to move and win. No stacks; bQ is uncovered with five filled sides, and wA1 and wA2 can run into the last one (-bL) and get arrows; wA3 is one of bQ's five neighbours, so it gets none. The hex gets a highlight.",
    setup: Setup::Position("A+slLmb3+(qM+A1+G1-A)4-(s-P-Qb3+a),w"),
    steps: &[Step::play(
        "Move a piece into the last empty hex next to the black Queen. It's highlighted, and the arrows show the pieces that can get there.",
        Reach(Surrounded("bQ")),
        "Pick any of the Ants the arrows start from. Ants can run all the way around the hive, right into the highlighted hex.",
    )
    .highlighted(&["-bL"])
    .with_arrows(&[("wA1", "-bL"), ("wA2", "-bL")])],
    outro: "That's the whole goal of Hive. If both Queens are surrounded by the same move, the game is a draw. Everything else in this tutorial is about getting there before your opponent does.",
};

const PLACING: Lesson = Lesson {
    id: "basics.placing",
    title: "Placing new pieces",
    intro: &[
        "The first two pieces of the game simply touch each other. From then on, every new piece must touch at least one of your own pieces and none of your opponent's.",
        "That's why the two armies grow apart at first: you can only add to your own side.",
        "Here both players opened with their Ladybug, a popular start. Ants are your most mobile pieces, so it usually pays to keep them in reserve for now and bring them in once you can see where they're needed.",
    ],
    brief: "From a 2000+ game (corpus game 10, after two moves each), White to move: Ladybug opening, Queens second.",
    setup: Setup::Position("L+Q1-l-q,w"),
    steps: &[
        Step::play(
            "Pick a Grasshopper from your reserve and place it. The board only offers the hexes where it may go.",
            Reach(Placed("wG")),
            "Choose a Grasshopper from your reserve, then a highlighted hex.",
        ),
        Step::read(
            "None of the hexes touching a black piece were on offer. That rule only applies to placing: once a piece is in play, it may move right up against the enemy.",
            &[],
        ),
    ],
    outro: "",
};

const QUEEN_RULE: Lesson = Lesson {
    id: "basics.queen_rule",
    title: "An army needs its Queen",
    intro: &[
        "Until your Queen is on the board, none of your pieces may move: you can only place new ones.",
        "She can't open the game, and she must arrive by your fourth turn.",
    ],
    brief: "From a 2000+ game (corpus game 1, after three moves each), White's fourth turn: wL, wM and wA1 placed, no wQ yet. Black's scripted reply bA1 bM- is legal wherever wQ lands; then any move of a white piece already on the board finishes the lesson.",
    setup: Setup::Position("AL+lm1-M3+q,w"),
    steps: &[
        Step::play(
            "It's your fourth turn and your Queen is still in your reserve. Place her.",
            Reach(Placed("wQ")),
            "On your fourth turn, only your Queen may be placed.",
        )
        .reply("bA1", "bM-"),
        Step::play(
            "Your Queen is in, so your pieces can move now. Click them to see, then move one.",
            MoveAny,
            "Move a piece that's already on the board instead of placing a new one.",
        ),
    ],
    outro: "Until she arrived, your army was frozen. Most players bring her in on their second or third turn.",
};

const ONE_HIVE: Lesson = Lesson {
    id: "basics.one_hive",
    title: "One Hive",
    intro: &[
        "This is the most important rule in Hive: the pieces must always form one connected group, and that includes while a piece is moving.",
        "To check a move, imagine taking the piece into your hand. If the rest falls apart into two groups, that piece can't move at all, even if it would land somewhere that joins everything back up.",
    ],
    brief: "From a 2000+ game (corpus game 2, after five moves each), White to move. wP and wG1 each hold part of the hive together (pinned); wA1 and wM are free, and wA1 can reach the black Queen.",
    setup: Setup::Position("AGl+qp+a1+P+Q1-M4+m,w"),
    steps: &[
        Step::read(
            "Click your pieces to see who can move. The ringed ones each hold part of the hive together: take one into your hand and the hive falls apart, so they can't move at all. Your Ant and your Mosquito hold nothing together and are free. Press Continue when you've looked around.",
            &["wP", "wG1"],
        ),
        Step::play(
            "Find a piece that can move and bring it next to the black Queen.",
            Reach(Touching("wA1", "bQ")),
            "Your Ant holds nothing together, so it is free to go.",
        ),
    ],
    outro: "A piece that holds the hive together is pinned. Pinning enemy pieces is one of the most important ideas in Hive, and it comes straight out of this rule.",
};

const GATES_AND_DOORS: Lesson = Lesson {
    id: "basics.gates_and_doors",
    title: "Gates and doors",
    intro: &[
        "Two pieces with one empty hex between them can form a gate or a door.",
        "If their corners point at each other, it's a gate: nothing can slide through. If their flat sides face each other, it's a door: there's room to slide through.",
    ],
    brief: "hivegame.com game 3f1c6882 (2016 vs 2199), White to move at ply 15. Ring 1 (bL-) is a gate wA1 cannot enter; ring 2 (/wP) is a door between wP and bA3 (flat sides facing). wA1 slides into the door; wA1 bL- and wA1 bP- are illegal.",
    setup: Setup::Position("A+a+p-qlL-Q+a2-a4-MP,w"),
    steps: &[Step::play(
        "1 is a gate: corners pointing at each other. 2 is a door: flat sides facing. Click your Ant to see which one it can reach, then slide it through the door.",
        Move {
            piece: "wA1",
            to: "/wP",
        },
        "Your Ant can't get into 1. Move it into 2.",
    )
    .marked(&["bL-", "/wP"])
    .with_arrows(&[("wA1", "/wP")])],
    outro: "Gates trap walking pieces, Queens included. Doors let them through.",
};

const FREEDOM_TO_MOVE: Lesson = Lesson {
    id: "basics.freedom_to_move",
    title: "Freedom to move",
    intro: &[
        "Walking pieces slide along the ground, so they only fit through an opening that is wide enough.",
        "A narrow opening with pieces on both sides is a gate: nothing can slide through it.",
    ],
    brief: "From a game between two 1900+ players (corpus game 8189, after 8 moves each), White to move. The slot wL- is walled in; wQ and wA3 touch it but both flanks of their way in are taken, so neither can enter (neither is pinned). wQ steps to \\wL, freeing one flank; then wA3 slides into the slot.",
    setup: Setup::Position("A+A+A+Q+L+B-a6=m5-(l+qp1-aM),w"),
    steps: &[
        Step::play(
            "Your Queen and Ant can't slide into the ringed hex: it's gated. Move your Queen along the arrow to open it.",
            Move {
                piece: "wQ",
                to: "\\wL",
            },
            "Click your Queen, then the hex the arrow points to.",
        )
        .marked(&["wL-"])
        .with_arrows(&[("wQ", "\\wL")])
        .reply("bA3", "-bP"),
        Step::play(
            "The gate is open. Slide your Ant into the highlighted hex.",
            Move {
                piece: "wA3",
                to: "wL-",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["wL-"])
        .with_arrows(&[("wA3", "wL-")]),
    ],
    outro: "Gates stop the Ant, the Spider and the Queen. Jumpers and climbers ignore them, as you'll see in the next chapter.",
};
