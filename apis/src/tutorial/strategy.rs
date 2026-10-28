use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "strategy",
    title: "First strategy",
    summary: "Racing, pinning, gating and shutting out: the ideas that decide most beginner games.",
    lessons: &[RACE, PIN, GATE, TWO_FOR_ONE, SHUTOUT, BUGZWANG, CONTROL],
};

const RACE: Lesson = Lesson {
    id: "strategy.race",
    title: "Counting the race",
    intro: &[
        "When both players go all-in on the enemy Queen, the game becomes a race, and races are decided by counting.",
        "Before each move, count how many turns you need to finish and how many your opponent needs. Count only moves that can really happen: a piece that is stuck, or too far away, doesn't count. One turn ahead is all it takes.",
    ],
    brief: "From a 2000+ game (corpus game 1022, after 15 moves each), White to move. Both Queens have four of six sides filled. wM, wG1 or wL to bP- fills a fifth side of bQ while bM stays pinned and bQ stuck; moves to -wA3 free bM, and wB2 bP- frees bQ. Scripted reply: bA2 to wA2\\, filling a fifth side of wQ.",
    setup: Setup::Position("G+Bq+paM3=B2+A+L2-(mPAa2+Aa2-(Qs1=b1-s)),w"),
    steps: &[
        Step::play(
            "Both Queens have four sides filled, and it's your move: you're a turn ahead. Fill a fifth side of the black Queen without setting any black piece free.",
            Reach(All(&[
                KillSpotsFilled {
                    queen: Color::Black,
                    at_least: 5,
                },
                Pinned("bM"),
                Stuck("bQ"),
            ])),
            "Fill a fifth side of the black Queen, but keep every black piece held down.",
        )
        .punished_by(&[("bM", "-bB1"), ("bQ", "-bB1")])
        .reply("bA2", "wA2\\"),
        Step::play(
            "Black answered by filling a fifth side of your Queen, but you're still one move ahead. Finish it.",
            Reach(Surrounded("bQ")),
            "One empty hex is left around the black Queen. Fill it.",
        ),
    ],
    outro: "Won by a single turn. The rest of this chapter is about winning races before they start: if your opponent's pieces can't move, they can't race.",
};

const PIN: Lesson = Lesson {
    id: "strategy.pin",
    title: "The pin",
    intro: &["If your piece is the only thing an enemy piece holds on to, that enemy piece can't move: moving it would split the hive. That's a pin."],
    brief: "hivegame.com game c03f467d (1941 vs 1653), White to move at ply 7. White played wA1 bA1-, hanging on bA1 alone. Closing read rings bA1.",
    setup: Setup::Position("ALpqa1+Q,w"),
    steps: &[
        Step::play(
            "Pin the black Ant.",
            Move {
                piece: "wA1",
                to: "bA1-",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bA1-"])
        .with_arrows(&[("wA1", "bA1-")]),
        Step::read("Pinned: it can't move as long as your Ant stays.", &["bA1"]),
    ],
    outro: "Pinning their Ants is especially strong: they're usually their best movers.",
};

const GATE: Lesson = Lesson {
    id: "strategy.gate",
    title: "The gate",
    intro: &["A gate is an empty hex with pieces on both sides of the way in. Walking pieces can't slide through it."],
    brief: "hivegame.com game 5bb74924 (1858 vs 1612), White to move at ply 15. White played wA1 bQ-; bQ isn't pinned but has no moves: both her empty neighbours (-bM, bA1-) are gated. Closing read rings them.",
    setup: Setup::Position("A+P+lb-a+M2-Q+m3+aA3-q,w"),
    steps: &[
        Step::play(
            "Move your Ant next to the black Queen.",
            Move {
                piece: "wA1",
                to: "bQ-",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bQ-"])
        .with_arrows(&[("wA1", "bQ-")]),
        Step::read("Both ringed hexes are gates: the Queen can't move at all.", &["-bM", "bA1-"]),
    ],
    outro: "A Queen who can't walk away is a Queen you can surround at your own pace.",
};

const TWO_FOR_ONE: Lesson = Lesson {
    id: "strategy.two_for_one",
    title: "Two for one",
    intro: &["The best pins hold more than one piece."],
    brief: "hivegame.com game 8cf91b2e (2225 vs 1973), White to move at ply 13. White played wL bA1/, pinning both bM and bA1. Closing read rings both.",
    setup: Setup::Position("M+pql+a3-(mAQ1+L-P),w"),
    steps: &[
        Step::play(
            "Move your Ladybug to the highlighted hex.",
            Move {
                piece: "wL",
                to: "bA1/",
            },
            "Click the Ladybug the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bA1/"])
        .with_arrows(&[("wL", "bA1/")]),
        Step::read("One move, two black pieces pinned.", &["bM", "bA1"]),
    ],
    outro: "",
};

const SHUTOUT: Lesson = Lesson {
    id: "strategy.shutout",
    title: "Shutting them out",
    intro: &[
        "In Hive there is no stalemate. If a player can't move or place anything, they simply pass, again and again, while you keep playing.",
        "Pin and gate enough of your opponent's pieces and you get free moves until you win.",
    ],
    brief: "From a 2000+ game (corpus game 507, after 23 moves each), White to move. Black has placed every piece; wA1 to bQ\\ takes away Black's last legal move.",
    setup: Setup::Position("A+a+g+GLgP2-(gpq1=B)3-aA5+(Qa-bG1=b1=B1+mA)5-ls-sM,w"),
    steps: &[Step::play(
        "Black has placed every piece, and only a few of them can still move. Find the move that takes away Black's last option, and Black will have to pass.",
        Reach(ShutOut(Color::Black)),
        "Click Black's pieces to see which ones can still move, then block the last one.",
    )],
    outro: "Black has nothing left to place and nothing that can move. From here every move is yours.",
};

const BUGZWANG: Lesson = Lesson {
    id: "strategy.bugzwang",
    title: "Bugzwang",
    intro: &["You may only pass when you have no legal move at all. If every move you have hurts you, you still have to play one."],
    brief: "hivegame.com game e8a3e095 (1989 vs 1628), White to move at ply 25. After wM -bA2 no black piece can move and Black's only spawn is next to bQ; Black placed bG1 -bQ, filling its own fifth kill spot; White then won with wL -bA3 (the learner may use any winning move).",
    setup: Setup::Position("A+a+ba1-M3-(pqbL1-(Ql+a-A1-P)2+mA),w"),
    steps: &[
        Step::play(
            "Move your Mosquito to the highlighted hex.",
            Move {
                piece: "wM",
                to: "-bA2",
            },
            "Click the Mosquito the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["-bA2"])
        .with_arrows(&[("wM", "-bA2")])
        .reply("bG1", "-bQ"),
        Step::play(
            "No black piece could move, and the only hex Black could place on touches its own Queen, so Black had to fill it. Now finish the game.",
            Reach(All(&[Surrounded("bQ"), Not(&Surrounded("wQ"))])),
            "One empty hex is left next to the black Queen.",
        )
        .marked(&["bG1"]),
    ],
    outro: "Take away your opponent's useful moves, and the moves they have left start doing your work for you.",
};

const CONTROL: Lesson = Lesson {
    id: "strategy.control",
    title: "Free pieces, stuck pieces",
    intro: &["Pin with cheap pieces and keep your Ants free. A free Ant can strike again next turn."],
    brief: "hivegame.com game 9361410a (1834 vs 1923), White to move at ply 9. White pinned bA1 with wM (bA1-) instead of an Ant; Black bA2 /bP; White's free wA1 then pinned bA2 (/bA2). Black bM -bP. Closing read rings both pinned Ants.",
    setup: Setup::Position("AM+P-l+a1+Q4-qp,w"),
    steps: &[
        Step::play(
            "Pin the black Ant with your Mosquito, not your Ant.",
            Move {
                piece: "wM",
                to: "bA1-",
            },
            "Click the Mosquito the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bA1-"])
        .with_arrows(&[("wM", "bA1-")])
        .reply("bA2", "/bP"),
        Step::play(
            "Black brought in another Ant. Yours is still free: pin it.",
            Move {
                piece: "wA1",
                to: "/bA2",
            },
            "Click your Ant, then the highlighted hex.",
        )
        .highlighted(&["/bA2"])
        .with_arrows(&[("wA1", "/bA2")]),
        Step::read("Both black Ants are pinned.", &["bA1", "bA2"]),
    ],
    outro: "That's the beginner's toolkit. Next: the tactics strong players use to actually finish a surround.",
};
