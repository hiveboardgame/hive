use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "basics",
    title: "The basics",
    summary: "How a turn works, how pieces enter the game, and the two rules every move obeys.",
    lessons: &[WELCOME, PLACING, QUEEN_RULE, MOVE_PIECES, ONE_HIVE, SURROUND, DRAW, PASS],
};

const WELCOME: Lesson = Lesson {
    id: "basics.welcome",
    title: "Welcome to Hivegame.com!",
    intro: &["Hive has no board: the pieces are the board. Every move must still keep the hive connected, and the winner is the player who surrounds the opponent's Queen Bee.",
        "When the game starts, all pieces are in the reserve. White goes first.",
    ],
    brief: "Empty board, White to move.",
    setup: Setup::Position(",w"),
    steps: &[
        Step::play(
            "This is how a game starts. You play as White. Place any piece from your reserve, except the Queen Bee [[wQ]].\nClick on the piece you choose in the reserve, then click on the highlighted hex.",
            Reach(All(&[AnyPlaced(Color::White), Not(&Placed("wQ"))])),
            "Click a piece in your reserve, then the highlighted hex. The Queen may not open the game.",
        )
        .next_player(Color::Black),
        Step::play(
            "Now play as Black. Place any piece from Black's reserve, except the Queen Bee [[bQ]], next to White's first piece.",
            Reach(All(&[AnyPlaced(Color::Black), Not(&Placed("bQ"))])),
            "Choose a Black piece other than the Queen Bee, then place it next to White's piece.",
        )
        .next_player(Color::White),
        Step::read(
            "Well done! You have placed the first piece for both White and Black.",
            &[],
        ),
    ],
    outro: "",
};

const PLACING: Lesson = Lesson {
    id: "basics.placing",
    title: "Placing new pieces",
    intro: &[
        "The first two pieces of the game have to touch each other.",
        "From now on, **every new piece you place must touch at least one of your own pieces and none of your opponent's pieces**.",
        "Here both players opened with their Ladybug, a popular start. Ants are a good piece to place next since they are very flexible",
    ],
    brief: "After both players open with Ladybug, White places any piece legally; Black replied by placing bQ on any legal hex.",
    setup: Setup::Position("L+l!,w"),
    steps: &[
        Step::play(
            "Play as White: pick any piece you like from the reserve and place it. New pieces entering the game cannot touch opponent pieces. The board will show the hexes available for placement.",
            PlaceAny,
            "Choose a piece from your reserve, then a highlighted hex.",
        )
        .reply_placement("bQ")
        .placement_crosses("bL"),
        Step::read(
            "Well done! None of the hexes touching a black piece were available to your piece. Remember this rule only applies to placing. Once a piece is in play, it may move also to hexes touching the opponent pieces.\n\nBlack replied to your move by placing their Queen Bee.",
            &[],
        )
        .placement_crosses("bL"),
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
    brief: "From a 2000+ game (corpus game 1, after three moves each), White's fourth turn: wL, wM and wA1 placed, no wQ yet. Black's scripted reply bA1 bM- is legal wherever wQ lands; placing the Queen and Black's reply complete this lesson. Movement follows in the next lesson.",
    setup: Setup::Position("AL+lm1-M3+q,w"),
    steps: &[
        Step::play(
            "The Queen Bee [[wQ]] has to be placed by your 4th turn. Put her in play now!",
            Reach(Placed("wQ")),
            "On your fourth turn, only your Queen may be placed.",
        )
        .reply("bA1", "bM-"),
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

const MOVE_PIECES: Lesson = Lesson {
    id: "basics.move_pieces",
    title: "Move your pieces",
    intro: &[
        "Your Queen Bee is now in play, so you can move pieces that are already on the hive.",
        "On each turn, you may move a piece in play or place a new piece from your reserve.",
    ],
    brief: "Continue the exact completed position of basics.queen_rule, after Black's Ant reply. When opened directly, use a representative completed position. Any legal movement of a White piece completes the lesson.",
    setup: Setup::AfterLesson {
        lesson: "basics.queen_rule",
        fallback: "MA+Q1-(Llm-a!2+q),w",
    },
    steps: &[
        Step::play(
            "You play as White. Click your pieces to see their legal moves, then move any piece already in play.",
            MoveAny,
            "Move a piece that is already on the board instead of placing a new one.",
        ),
        Step::read("Well done! You have moved a piece in play.", &[]),
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
    brief: "From a 2000+ game (corpus game 2, after five moves each), White to move. wP and wG1 each hold part of the hive together (pinned); wQ, wA1 and wM are free. Move a White piece, then a Black piece. On Black's turn mark bL and bQ only if they are actually immobile; White can free them with some moves. Placements do not count.",
    setup: Setup::Position("AGl+qp+a1+P+Q1-M4+m,w"),
    steps: &[
        Step::play(
            "The highlighted pieces cannot move without breaking the hive. You play as White. Your Queen Bee, Ant and Mosquito can move.

Click your pieces to inspect their moves, then move one of them. Don't worry if you don't know how pieces move yet, you'll learn soon!",
            MoveAny,
            "Move a piece that's already on the board instead of placing a new one.",
        )
        .marked(&["wP", "wG1"])
        .without_mark_numbers()
        .next_player(Color::Black),
        Step::play(
            "Now play as Black. Any highlighted pieces cannot move in this position. Check your pieces, then make any legal move with a Black piece already in play.",
            MoveAny,
            "Move a Black piece that's already on the board instead of placing a new one.",
        )
        .marked_when_immobile(&["bL", "bQ"])
        .next_player(Color::White),
        Step::read("Well done! You have kept the hive connected while moving both colours.", &[])
            .with_win_prompt(Color::Black, "Good job, Black won!"),
    ],
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
            "That's the whole goal of Hive: surround your opponent's Queen Bee first.\n\nIt doesn't matter what colour the pieces surrounding the Queen Bee are. Here, the black Ladybug [[bL]] is actually hurting Black!",
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
    setup: Setup::SwapPieces {
        position: "A+A+MG+B+g!Qb+g3+P6-b+l-m7+a+L+q,w",
        first: "wP",
        second: "wA1",
    },
    steps: &[
        Step::play(
            "You play as White. Move your Grasshopper [[wG1]] to surround both Queen Bees at the same time.",
            Move {
                piece: "wG1",
                to: "wQ-",
            },
            "Move the white Grasshopper into the hole between the two Queen Bees.",
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


const PASS: Lesson = Lesson {
    id: "basics.pass",
    title: "Can I pass?",
    intro: &[
        "You may pass only when you have no legal move and no legal placement. If you can play, you must take your turn.",
        "Black has spent too many turns moving only a few pieces. White has blocked every place where a new Black piece could enter, and none of Black's pieces can move. This is called a **shut-out**.",
        "Black must pass, so White plays again. Black will keep passing only while it has no legal move or placement; a pass is never a choice when a move is available.",
    ],
    brief: "Supplied HOP A+ql+gA2+s+M3-L4-Q,w. Verify a real Black forced pass, then accept any legal White placement or movement. Show forbidden placement crosses around Black pieces. TODO part two: awaiting an authored position with one forced White move followed by a winning Black reply.",
    setup: Setup::ForcedPass {
        position: "A+ql+gA2+s+M3-L4-Q,w",
        color: Color::Black,
    },
    steps: &[
        Step::play(
            "Black has no legal move or placement and has passed. You play as White again: place any piece from your reserve or make any legal move.",
            PlayAny,
            "Make any legal move or placement as White.",
        )
        .placement_crosses_for(Color::Black),
    ],
    outro: "Well done! You used the extra turn. Black can play again as soon as a legal move or placement becomes available. Remember: you must play if you have a legal move, even when it leads to a loss.",
};
