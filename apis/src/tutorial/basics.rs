use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "basics",
    title: "The basics",
    summary: "How the game starts, how pieces enter the game, how to win and the One Hive Rule.",
    lessons: &[
        WELCOME,
        PLACING,
        QUEEN_RULE,
        MOVE_PIECES,
        ONE_HIVE,
        SURROUND,
        DRAW,
        PASS,
    ],
};

const WELCOME: Lesson = Lesson {
    id: "basics.welcome",
    title: "Welcome to Hive",
    intro: &["Hive has no board: the pieces form the playing area.\nSurround your opponent's Queen Bee [[bQ]] to win.",
        "Your unplayed pieces are your **reserve**.\nWhite goes first, then Black.\nPlayers take turns, placing or moving one piece each turn.",
    ],
    brief: "Empty board, White to move.",
    setup: Setup::Position(",w"),
    steps: &[
        Step::play(
            "You play as White. Start with any piece except the Queen Bee [[wQ]]\nTo make your first move, click a piece from the reserve, then the highlighted hex.",
            Reach(All(&[AnyPlaced(Color::White), Not(&Placed("wQ"))])),
            "Choose any piece from your reserve, except the Queen Bee, then click the highlighted hex.",
        )
        .next_player(Color::Black),
        Step::play(
            "Now play as Black. The Queen Bee [[bQ]] cannot start.\nPlace a piece from the reserve next to White's piece.",
            Reach(All(&[AnyPlaced(Color::Black), Not(&Placed("bQ"))])),
            "Place any black piece from the reserve, except the Queen Bee, next to White's piece.",
        )
        .next_player(Color::White),
        Step::read(
            "Well done! Both players have placed their first piece.",
            &[],
        ),
    ],
    outro: "",
};

const PLACING: Lesson = Lesson {
    id: "basics.placing",
    title: "Placing new pieces",
    intro: &[
        "The first two pieces must touch.",
        "After that, **new pieces must touch your own pieces and none of your opponent's**. This placement rule does not apply to movement",
        "Here, both players opened with a Ladybug [[wL]] [[bL]], a common start.",
    ],
    brief: "After both players open with Ladybug, White places any piece legally; Black replied by placing bQ on any legal hex.",
    setup: Setup::Position("L+l!,w"),
    steps: &[
        Step::play(
            "You play as White. New pieces cannot touch Black's pieces.\nChoose any piece from the reserve, then a highlighted hex.\nThe hexes available for placement are called **spawn points**.",
            PlaceAny,
            "Choose any piece from the reserve, then a highlighted hex.",
        )
        .reply_placement("bQ")
        .placement_crosses("bL"),
        Step::read(
            "Well done! Black replied by placing their Queen Bee [[bQ]]",
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
        "You cannot move your pieces until your Queen Bee [[wQ]] is in play.",
        "She cannot be your 1st piece and must be placed by your 4th turn.",
    ],
    brief: "From a 2000+ game (corpus game 1, after three moves each), White's fourth turn: wL, wM and wA1 placed, no wQ yet. Black's scripted reply bA1 bM- is legal wherever wQ lands; placing the Queen and Black's reply complete this lesson. Movement follows in the next lesson.",
    setup: Setup::Position("AL+lm1-M3+q,w"),
    steps: &[
        Step::play(
            "This is your 4th turn. Place your Queen Bee [[wQ]]",
            Reach(Placed("wQ")),
            "Place your Queen Bee. She must enter by your 4th turn.",
        )
        .reply("bA1", "bM-"),
        Step::read(
            "Your Queen Bee [[wQ]] is in play!\nFrom now on, you may also move your pieces.",
            &[],
        )
        .with_completion_tip(
            "Tip: Place your Queen Bee [[wQ]] early to make movement available.",
        ),
    ],
    outro: "",
};

const MOVE_PIECES: Lesson = Lesson {
    id: "basics.move_pieces",
    title: "Move your pieces",
    intro: &[
        "With your Queen Bee [[wQ]] in play, your pieces can move.",
        "Each turn, either move a piece in play or place a new piece from your reserve.",
    ],
    brief: "Continue the exact completed position of basics.queen_rule, after Black's Ant reply. When opened directly, use a representative completed position. Any legal movement of a White piece completes the lesson.",
    setup: Setup::AfterLesson {
        lesson: "basics.queen_rule",
        fallback: "MA+Q1-(Llm-a!2+q),w",
    },
    steps: &[
        Step::play(
            "You play as White.\nClick a piece already in play to see how it moves, then make any move.",
            MoveAny,
            "That placement would be allowed in a real game, but for this lesson, move a piece already in play.",
        ),
        Step::read("Well done! You moved your first piece!", &[]),
    ],
    outro: "",
};

const ONE_HIVE: Lesson = Lesson {
    id: "basics.one_hive",
    title: "One Hive Rule",
    intro: &[
        "The **One Hive Rule**: all pieces must stay connected throughout a move.",
        "A piece is **pinned** if removing it would split the hive into separate groups. A **pinned** piece cannot move.",
        "To check whether a piece can move, imagine picking the piece up.\nIf the remaining pieces (the hive) become disconnected, it cannot move. This applies even if its destination would reconnect the hive.",
    ],
    brief: "From a 2000+ game (corpus game 2, after five moves each), White to move. wP and wG1 each hold part of the hive together (pinned); wQ, wA1 and wM are free. Move a White piece, then a Black piece. On Black's turn mark bL and bQ only if they are actually immobile; White can free them with some moves. Placements do not count.",
    setup: Setup::Position("AGl+qp+a1+P+Q1-M4+m,w"),
    steps: &[
        Step::play(
            "You play as White. Highlighted pieces are pinned.\nClick your Queen Bee [[wQ]], Ant [[wA1]] or Mosquito [[wM]], then move one.",
            MoveAny,
            "Move a piece already in play; do not place a piece from the reserve.",
        )
        .marked(&["wP", "wG1"])
        .without_mark_numbers()
        .next_player(Color::Black),
        Step::play(
            "Now play as Black. Highlighted pieces cannot move.\nMove any black piece already in play.",
            MoveAny,
            "Move a black piece already in play.",
        )
        .marked_when_immobile(&["bL", "bQ"])
        .next_player(Color::White),
        Step::read("Well done! Both moves kept the hive connected.", &[])
            .with_win_prompt(Color::Black, "Black wins!"),
    ],
    outro: "",
};

const SURROUND: Lesson = Lesson {
    id: "basics.surround",
    title: "How to Win",
    intro: &[
        "Surround the opponent's Queen Bee [[bQ]] on all six sides to win.\nPieces of either colour count.",
        "Here, the black Queen [[bQ]] has just one empty [green]neighbouring hex[/green].",
    ],
    brief: "From a 2000+ game (corpus game 420, after 14 moves each), White to move and win. No stacks; bQ is uncovered with five filled sides, and wA1 and wA2 can run into the last one (-bL). The winning hex is highlighted without arrows.",
    setup: Setup::Position("A+slLmb3+(qM+A1+G1-A)4-(s-P-Qb3+a),w"),
    steps: &[
        Step::play(
            "You play as White.\nMove an Ant [[wA1]] into the [green]green hex[/green] to surround the black Queen [[bQ]].",
            Reach(Surrounded("bQ")),
            "It doesn't surround the black Queen! [[bQ]]. Move a white Ant [[wA1]] into the [green]green hex[/green].",
        )
        .highlighted(&["-bL"]),
        Step::read(
            "Well done! Even Black's Ladybug [[bL]] helped surround the black Queen [[bQ]]\nThe surrounding pieces can be either colour.",
            &[],
        )
        .with_completion_tip(
            "Tip: Be careful what you place beside your own Queen Bee [[wQ]]",
        ),
    ],
    outro: "",
};

const DRAW: Lesson = Lesson {
    id: "basics.draw",
    title: "Draw?",
    intro: &[
        "If both Queen Bees [[wQ]] [[bQ]] become surrounded at the same time, the game is a draw.",
        "Here, the [green]green hex [/green]is the last empty space adjacent to both Queen Bees.",
    ],
    brief: "White to move. Only wG1 to wQ- completes the lesson by surrounding both Queens at the same time; every other legal move is rejected.",
    setup: Setup::SwapPieces {
        position: "A+A+MG+B+g!Qb+g3+P6-b+l-m7+a+L+q,w",
        first: "wP",
        second: "wA1",
    },
    steps: &[
        Step::play(
            "You play as White.\nJump your Grasshopper [[wG1]] into the [green]green hex [/green]to surround both Queen Bees [[wQ]] [[bQ]]",
            Move {
                piece: "wG1",
                to: "wQ-",
            },
            "Move the white Grasshopper into the hole between the Queen Bees.",
        )
        .highlighted(&["wQ-"]),
        Step::read(
            "Both Queen Bees [[wQ]] [[bQ]] are surrounded at the same time. It's a draw!",
            &[],
        )
        .with_completion_tip(
            "Tip: When the Queen Bees are close to each other, it's more likely to have a draw.",
        ),
    ],
    outro: "",
};

const PASS: Lesson = Lesson {
    id: "basics.pass",
    title: "Can I pass?",
    intro: &[
        "You **must pass** if you have no legal move or placement.\nHowever, **if you have any move available** (placement or movement), **you must play**.",
        "Black cannot move any piece without breaking the hive, and White has blocked all of Black's spawn points.\nThis situation is called **shut-out**.",
        "Black must pass, so White plays again.",
    ],
    brief: "Supplied HOP A+ql+gA2+s+M3-L4-Q,w. Verify a real Black forced pass, then require two consecutive White placements while Black remains shut out. Show forbidden placement crosses around Black pieces. TODO part two: awaiting an authored position with one forced White move followed by a winning Black reply.",
    setup: Setup::ForcedPass {
        position: "A+ql+gA2+s+M3-L4-Q,w",
        color: Color::Black,
    },
    steps: &[
        Step::play(
            "Black has passed. You play as White again.\nPlace any piece from the reserve.",
            PlaceAny,
            "Place a piece from the reserve so Black stays shut out.",
        )
        .placement_crosses_for(Color::Black)
        .your_turn(),
        Step::play(
            "Black must pass again.\nPlace another white piece from the reserve.",
            PlaceAny,
            "Place another white piece from the reserve.",
        )
        .placement_crosses_for(Color::Black),
        Step::read("Well done! Black passed twice, so White was able to place two pieces in a row.", &[])
            .placement_crosses_for(Color::Black),
    ],
    outro: "Black can play again as soon as a legal move or placement becomes available.",
};
