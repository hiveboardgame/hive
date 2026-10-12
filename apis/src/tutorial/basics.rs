use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "basics",
    title: "The basics",
    summary: "How a turn works, how pieces enter the game, and the two rules every move obeys.",
    lessons: &[WELCOME, PLACING, QUEEN_RULE, ONE_HIVE, SURROUND, DRAW],
};

const WELCOME: Lesson = Lesson {
    id: "basics.welcome",
    title: "Welcome to Hive!",
    intro: &["Hive has no board: the pieces are the board. Every move must still keep the hive connected, and the winner is the player who surrounds the opponent's Queen Bee.",
        "When the game starts, all pieces are in the reserve. White goes first.",
    ],
    brief: "Empty board, White to move.",
    setup: Setup::Position(",w"),
    steps: &[
        Step::play(
            "You play as White. Place any piece from your reserve, except the Queen Bee [[wQ]].\nClick on the piece you choose in the reserve, then click on the highlighted hex.",
            Reach(All(&[AnyPlaced(Color::White), Not(&Placed("wQ"))])),
            "Click a piece in your reserve, then the highlighted hex. The Queen may not open the game.",
        )
        .reply_placement("bL"),
        Step::read(
            "Black replied by placing a Ladybug next to your piece.",
            &[],
        ),
    ],
    outro: "Well done, you played your first move! Black has replied by placing their first piece too, next to yours.",
};

const PLACING: Lesson = Lesson {
    id: "basics.placing",
    title: "Placing new pieces",
    intro: &[
        "The first two pieces of the game have to touch each other.",
        "From now on, **every new piece you place must touch at least one of your own pieces and none of your opponent's pieces**.",
        "Here both players opened with their Ladybug, a popular start. Ants are a good piece to place next since they are very flexible",
    ],
    brief: "After both players open with Ladybug, White places wA1; Black replied by placing bQ on any legal hex.",
    setup: Setup::Position("L+l!,w"),
    steps: &[
        Step::play(
            "Play as White: pick an ant ([[wA1]]) from your reserve and place it. The board will show the hexes available for placement.",
            Reach(Placed("wA1")),
            "Choose an Ant from your reserve, then a highlighted hex.",
        )
        .reply_placement("bQ"),
        Step::read(
            "Well done! None of the hexes touching a black piece were available to your Ant. Remember this rule only applies to placing. Once a piece is in play, it may move also to hexes touching the opponent pieces.\n\nBlack replied to your move by placing their Queen Bee.",
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
        "She cannot open the game, and she must arrive by your 4th turn.",
    ],
    brief: "From a 2000+ game (corpus game 1, after three moves each), White's fourth turn: wL, wM and wA1 placed, no wQ yet. Black's scripted reply bA1 bM- is legal wherever wQ lands; then any move of a white piece already on the board finishes the lesson.",
    setup: Setup::Position("AL+lm1-M3+q,w"),
    steps: &[
        Step::play(
            "It's your 4th turn and your Queen Bee is still in your reserve. Put her in play.",
            Reach(Placed("wQ")),
            "On your fourth turn, only your Queen may be placed.",
        )
        .reply("bA1", "bM-"),
        Step::play(
            "Your Queen Bee is in play, so your pieces can move now! Click them to see where they can go, then move one.",
            MoveAny,
            "Move a piece that's already on the board instead of placing a new one.",
        ),
        Step::read(
            "Once your Queen Bee is in play, you can choose on every turn whether to move a piece already in play or place a new one.",
            &[],
        )
        .with_completion_tip(
            "Tip: Bring your Queen Bee into play as soon as possible so you can move your pieces early.",
        ),
    ],
    outro: "",
};

const ONE_HIVE: Lesson = Lesson {
    id: "basics.one_hive",
    title: "One Hive Rule",
    intro: &[
        "This is the most important rule in Hive: **all pieces in play must form one connected group at all times**.",
        "A piece cannot move if moving it would break the hive into two or more groups.",
        "To check whether a move is legal, imagine taking the piece you want to move into your hand. If the rest of the hive breaks into two or more disconnected groups, that move is illegal, even if the piece could land somewhere that would reconnect the hive.",
    ],
    brief: "From a 2000+ game (corpus game 2, after five moves each), White to move. wP and wG1 each hold part of the hive together (pinned); wQ, wA1 and wM are free. Any legal movement completes the lesson; placing a reserve piece does not.",
    setup: Setup::Position("AGl+qp+a1+P+Q1-M4+m,w"),
    steps: &[Step::play(
        "The highlighted pieces cannot move without breaking the hive. Your Queen Bee, Ant, and Mosquito are able to move.\n\nCheck for yourself by clicking on your pieces, then move any of them to complete the lesson.",
        MoveAny,
        "Move a piece that's already on the board instead of placing a new one.",
    )
    .marked(&["wP", "wG1"])
    .without_mark_numbers()],
    outro: "A piece that holds the hive together is pinned. Pinning enemy pieces is one of the most important ideas in Hive, and it comes straight out of this rule: every move must preserve one connected hive.",
};

const SURROUND: Lesson = Lesson {
    id: "basics.surround",
    title: "How to Win",
    intro: &[
        "The goal in Hive is to surround the opponent's Queen Bee on all six sides. Both your pieces and your opponent's pieces count.",
        "This is the end of a real game between two strong players. The black Queen has pieces on five of her six sides, and it's White's move.",
    ],
    brief: "From a 2000+ game (corpus game 420, after 14 moves each), White to move and win. No stacks; bQ is uncovered with five filled sides, and wA1 and wA2 can run into the last one (-bL). The winning hex is highlighted without arrows.",
    setup: Setup::Position("A+slLmb3+(qM+A1+G1-A)4-(s-P-Qb3+a),w"),
    steps: &[
        Step::play(
            "Move one of your Ants into the highlighted hex to completely surround the black Queen Bee.",
            Reach(Surrounded("bQ")),
            "Move one of your Ants into the highlighted hex.",
        )
        .highlighted(&["-bL"]),
        Step::read(
            "That's the whole goal of Hive: surround your opponent's Queen Bee first.\n\nIt doesn't matter what colour the pieces surrounding the Queen Bee are. Here, the black Ladybug is actually hurting Black!",
            &[],
        )
        .with_completion_tip(
            "Tip: Be careful which pieces you place next to your own Queen Bee!",
        ),
    ],
    outro: "",
};

const DRAW: Lesson = Lesson {
    id: "basics.draw",
    title: "Draw?",
    intro: &[
        "Games can also end in a draw. The most common way this happens is when both Queen Bees are surrounded at the same time.",
        "In this game, the highlighted hex is crucial: by moving a piece there, both Queen Bees lose their last hex of freedom!",
    ],
    brief: "White to move. Only wG1 to wQ- completes the lesson by surrounding both Queens at the same time; every other legal move is rejected.",
    setup: Setup::Position("A+A+MG+B+g!Qb+g3+P6-b+l-m7+a+L+q,w"),
    steps: &[
        Step::play(
            "You play as White. Move your Grasshopper [[wG1]] to surround both Queen Bees.",
            Move {
                piece: "wG1",
                to: "wQ-",
            },
            "Not quite. Move the white Grasshopper into the hole between the two Queen Bees.",
        )
        .highlighted(&["wQ-"]),
        Step::read(
            "Well done! If you hadn't done it, your opponent would have forced the draw.",
            &[],
        )
        .with_completion_tip(
            "Tip: When the Queen Bees are very close to each other, a draw is more likely!",
        ),
    ],
    outro: "",
};
