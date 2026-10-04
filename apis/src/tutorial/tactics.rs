use super::{
    goal::Goal::*,
    lesson::{Chapter, Lesson, Setup, Step},
};

pub const CHAPTER: Chapter = Chapter {
    id: "tactics",
    title: "Tactics",
    summary: "Kill spots, true pins, qualifying for the win, direct drops, and how to beat (and build) the defences around a Queen.",
    lessons: &[
        KILL_SPOTS,
        TRUE_AND_FALSE_PINS,
        DOUBLE_PIN,
        PIN_REPLACEMENT,
        QUALIFYING,
        BEETLE_ON_QUEEN,
        DIRECT_DROPS,
        RECOVERY,
        BEETLE_ON_PILLBUG,
        FLOOD_PILLBUG,
        REMOVE_PILLBUG,
        PROXIMITY_PILLBUG,
        QUEEN_CHOKE,
        CAVERN,
        ANTI_SPAWN,
    ],
};

const KILL_SPOTS: Lesson = Lesson {
    id: "tactics.kill_spots",
    title: "Kill spots",
    intro: &["The six hexes around a Queen are her kill spots. Fill all six to win. Any piece counts, even her own."],
    brief: "hivegame.com game fb745694 (2289 vs 1951), White to move at ply 9. White played wA1 bQ\\, filling a fourth kill spot. Closing read numbers the four filled spots.",
    setup: Setup::Position("A+QA2-(Glq+p2+m),w"),
    steps: &[
        Step::play(
            "Fill one more of the black Queen's kill spots.",
            Move {
                piece: "wA1",
                to: "bQ\\",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bQ\\"])
        .with_arrows(&[("wA1", "bQ\\")]),
        Step::read("Four filled, two to go.", &["bM", "wA1", "bP", "bL"]),
    ],
    outro: "",
};

const TRUE_AND_FALSE_PINS: Lesson = Lesson {
    id: "tactics.true_and_false_pins",
    title: "True pins",
    intro: &["Pin a piece next to the enemy Queen in a straight line, and no empty hex touches all three pieces. Nothing can free it: that's a true pin."],
    brief: "hivegame.com game e240ed9d (1805 vs 1827), White to move at ply 7. White played wA1 -bM: bQ, bM and wA1 in one line. Closing read numbers the three.",
    setup: Setup::Position("A+L+lm2-Q3-q,w"),
    steps: &[
        Step::play(
            "Pin the black Mosquito in line with its Queen.",
            Move {
                piece: "wA1",
                to: "-bM",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["-bM"])
        .with_arrows(&[("wA1", "-bM")]),
        Step::read("Queen, Mosquito, Ant: one straight line.", &["bQ", "bM", "wA1"]),
    ],
    outro: "A pin at an angle is weaker: the empty hex in the corner touches all three, and filling it frees the pinned piece.",
};

const DOUBLE_PIN: Lesson = Lesson {
    id: "tactics.double_pin",
    title: "The double pin",
    intro: &["Pin the same piece from two sides. Breaking one pin is no longer enough to free it."],
    brief: "hivegame.com game 2752c727 (1886 vs 1789), White to move at ply 21. wM already hangs on bG2; White played wA2 -bG2 for a second, independent pin. Closing read rings bG2 and both pinners.",
    setup: Setup::Position("A+Q-Pgm+A5=B5=b4+l5-(q+p1-g+M),w"),
    steps: &[
        Step::play(
            "Your Mosquito pins the black Grasshopper. Pin it a second time.",
            Move {
                piece: "wA2",
                to: "-bG2",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .marked(&["bG2"])
        .highlighted(&["-bG2"])
        .with_arrows(&[("wA2", "-bG2")]),
        Step::read("Two pins on one piece.", &["bG2", "wM", "wA2"]),
    ],
    outro: "",
};

const PIN_REPLACEMENT: Lesson = Lesson {
    id: "tactics.pin_replacement",
    title: "Pin replacement",
    intro: &["Pinning with an Ant ties down your best piece. Swap in a cheaper piece to hold the pin, and the Ant is free again."],
    brief: "hivegame.com game 68a366d6 (1929 vs 1880), White to move at ply 17. White played wS1 bA2- (the Spider takes over the pin on bA2), Black bP -wA2, then wA2 bA1\\ (the Ant leaves). Closing read rings bA2 and wS1.",
    setup: Setup::Position("A+L+Q1-a2-(lmM-aA-S1-q+p),w"),
    steps: &[
        Step::play(
            "Bring your Spider next to the pinned black Ant.",
            Move {
                piece: "wS1",
                to: "bA2-",
            },
            "Click the Spider the arrow starts from, then the highlighted hex.",
        )
        .marked(&["bA2"])
        .highlighted(&["bA2-"])
        .with_arrows(&[("wS1", "bA2-")])
        .reply("bP", "-wA2"),
        Step::play(
            "The Spider holds the pin now. Your Ant is free: use it.",
            Move {
                piece: "wA2",
                to: "bA1\\",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["bA1\\"])
        .with_arrows(&[("wA2", "bA1\\")]),
        Step::read("Still pinned, and your Ant is at work elsewhere.", &["bA2", "wS1"]),
    ],
    outro: "",
};

const QUALIFYING: Lesson = Lesson {
    id: "tactics.qualifying",
    title: "Qualifying for the win",
    intro: &["A Pillbug next to its own Queen can lift her out of a surround. Until you deal with it, filling kill spots doesn't win."],
    brief: "hivegame.com game 4ba9f2d7 (1838 vs 1827), White to move at ply 13. White placed wB1 \\wM, a fifth kill spot; Black answered bQ bP/: the Pillbug lifted the Queen out. Closing read rings bQ and bP.",
    setup: Setup::Position("Mpm1-(qLAa1+l1-Q),w"),
    steps: &[
        Step::play(
            "Place a Beetle on the highlighted hex: a fifth kill spot.",
            Move {
                piece: "wB1",
                to: "\\wM",
            },
            "Choose a Beetle from your reserve, then the highlighted hex.",
        )
        .marked(&["bP"])
        .highlighted(&["\\wM"])
        .reply("bQ", "bP/"),
        Step::read(
            "The ringed Pillbug lifted its Queen out. Five filled counted for nothing.",
            &["bQ", "bP"],
        ),
    ],
    outro: "The next lessons show how to qualify: cover the Queen, cover or remove the Pillbug, or choke the Queen.",
};

const BEETLE_ON_QUEEN: Lesson = Lesson {
    id: "tactics.beetle_on_queen",
    title: "Beetle on the Queen",
    intro: &["A Beetle on the enemy Queen pins her down, and the stack counts as yours. Now you may place right next to her."],
    brief: "hivegame.com game 2184e34b (1895 vs 2205), White to move at ply 15. White played wB1 bQ, Black bA1 wA1\\, then White dropped wM wB1/ straight into a kill spot.",
    setup: Setup::Position("A+QPg+a3=b3-(a+q-A3=B),w"),
    steps: &[
        Step::play(
            "Climb onto the black Queen.",
            Move {
                piece: "wB1",
                to: "bQ",
            },
            "Click the Beetle the arrow starts from, then the black Queen.",
        )
        .highlighted(&["bQ"])
        .with_arrows(&[("wB1", "bQ")])
        .reply("bA1", "wA1\\"),
        Step::play(
            "Place your Mosquito straight into her kill spot.",
            Move {
                piece: "wM",
                to: "wB1/",
            },
            "Choose your Mosquito from your reserve, then the highlighted hex.",
        )
        .highlighted(&["wB1/"]),
    ],
    outro: "Keep the Beetle on top until only one kill spot is left. Then it steps down for the win.",
};

const DIRECT_DROPS: Lesson = Lesson {
    id: "tactics.direct_drops",
    title: "Direct drops",
    intro: &["Once every piece next to a kill spot is yours, you can place straight into it. That saves a whole move."],
    brief: "hivegame.com game 38fd6ffc (2349 vs 1729), White to move at ply 17. wB1 covers bQ; White placed wB2 /wB1 straight into a kill spot.",
    setup: Setup::Position("A+QP-b+p2=b2+l4-(q+A1=B),w"),
    steps: &[
        Step::play(
            "Your Beetle covers the black Queen. Drop your other Beetle into her kill spot.",
            Move {
                piece: "wB2",
                to: "/wB1",
            },
            "Choose a Beetle from your reserve, then the highlighted hex.",
        )
        .marked(&["wB1"])
        .highlighted(&["/wB1"]),
        Step::read("Placed straight into a kill spot: no walking needed.", &["wB2"]),
    ],
    outro: "",
};

const RECOVERY: Lesson = Lesson {
    id: "tactics.recovery",
    title: "Recovery",
    intro: &["A black Beetle on your Queen lets Black drop pieces next to her. Climb on top of it and the stack is white again."],
    brief: "hivegame.com game 9646ce91 (1956 vs 1674), White to move at ply 15. bB1 covers wQ; White played wM bB1 (the Mosquito copies a Beetle and climbs on). Black drop spots next to wQ go from 1 to 0.",
    setup: Setup::Position("LM1+(p+q2=B)1-(Qa1=b),w"),
    steps: &[
        Step::play(
            "Climb your Mosquito onto the Beetle covering your Queen.",
            Move {
                piece: "wM",
                to: "bB1",
            },
            "Click the Mosquito the arrow starts from, then the black Beetle.",
        )
        .marked(&["bB1"])
        .highlighted(&["bB1"])
        .with_arrows(&[("wM", "bB1")]),
        Step::read(
            "The stack is white again: Black can't drop next to your Queen.",
            &["wM"],
        ),
    ],
    outro: "",
};

const BEETLE_ON_PILLBUG: Lesson = Lesson {
    id: "tactics.beetle_on_pillbug",
    title: "Beetle on the Pillbug",
    intro: &["A covered Pillbug can't move and can't lift anything, so it can't rescue its Queen."],
    brief: "hivegame.com game 95ce0261 (1965 vs 2137), White to move at ply 17. bP touches bQ; White played wB2 bP.",
    setup: Setup::Position("A+B-a1-(q+p1=B1-l-PQa+a),w"),
    steps: &[
        Step::play(
            "Cover the black Pillbug next to its Queen.",
            Move {
                piece: "wB2",
                to: "bP",
            },
            "Click the Beetle the arrow starts from, then the black Pillbug.",
        )
        .marked(&["bQ"])
        .highlighted(&["bP"])
        .with_arrows(&[("wB2", "bP")]),
        Step::read("The Pillbug is out of action.", &["wB2"]),
    ],
    outro: "",
};

const FLOOD_PILLBUG: Lesson = Lesson {
    id: "tactics.flood_pillbug",
    title: "Flooding the Pillbug",
    intro: &["A Pillbug needs an empty hex next to it to put something down. Fill them all and it can't lift anything."],
    brief: "hivegame.com game a9e39e2f (2037 vs 1809), White to move at ply 29. bP has one empty neighbour; White played wL -wB1 into it.",
    setup: Setup::Position("AQ+PABL2=m1+(qp+SB-a1+b)1-l5+M,w"),
    steps: &[
        Step::play(
            "Take away the black Pillbug's last empty hex.",
            Move {
                piece: "wL",
                to: "-wB1",
            },
            "Click the Ladybug the arrow starts from, then the highlighted hex.",
        )
        .marked(&["bP"])
        .highlighted(&["-wB1"])
        .with_arrows(&[("wL", "-wB1")]),
        Step::read("No room left around the Pillbug.", &["bP"]),
    ],
    outro: "",
};

const REMOVE_PILLBUG: Lesson = Lesson {
    id: "tactics.remove_pillbug",
    title: "Removing the Pillbug",
    intro: &["Your Pillbug can lift enemy pieces too, including their Pillbug. Throw it away from its Queen."],
    brief: "hivegame.com game 8d083df2 (1815 vs 1673), White to move at ply 9. bP touches bQ; wP throws it to bB1-.",
    setup: Setup::Position("A+qp-P+Q2-b,w"),
    steps: &[
        Step::play(
            "Use your Pillbug to throw the black Pillbug away from its Queen.",
            Move {
                piece: "bP",
                to: "bB1-",
            },
            "Click the black Pillbug, then the highlighted hex.",
        )
        .marked(&["bQ"])
        .highlighted(&["bB1-"])
        .with_arrows(&[("bP", "bB1-")]),
        Step::read("The Queen has lost her rescuer.", &["bQ", "bP"]),
    ],
    outro: "",
};

const PROXIMITY_PILLBUG: Lesson = Lesson {
    id: "tactics.proximity_pillbug",
    title: "The proximity Pillbug",
    intro: &["A Pillbug two hexes from its Queen still touches two of her kill spots, and it's much harder to pin or cover there."],
    brief: "hivegame.com game a2a964a9 (2141 vs 1795), White to move at ply 19. wP doesn't touch wQ; it throws bB2 out of her kill spot to -bB1.",
    setup: Setup::Position("A+m+q+A1=B2-(p+b+Pb-Q3-l),w"),
    steps: &[
        Step::play(
            "Your ringed Pillbug doesn't touch your Queen. Throw the black Beetle out of her kill spot.",
            Move {
                piece: "bB2",
                to: "-bB1",
            },
            "Click the black Beetle next to your Queen, then the highlighted hex.",
        )
        .marked(&["wP"])
        .highlighted(&["-bB1"])
        .with_arrows(&[("bB2", "-bB1")]),
        Step::read("One kill spot cleared, from a distance.", &["wQ", "wP"]),
    ],
    outro: "",
};

const QUEEN_CHOKE: Lesson = Lesson {
    id: "tactics.queen_choke",
    title: "Choking the Queen",
    intro: &["If the Queen can't move and Black can't place next to her, no Pillbug can ever arrive to save her."],
    brief: "hivegame.com game ef712644 (1848 vs 1906), White to move at ply 9. White played wM /bQ: bQ has no moves and no hex next to her is open to Black placements. Closing read rings her empty hexes.",
    setup: Setup::Position("AM-Q1-(P-lq2+a2-a),w"),
    steps: &[
        Step::play(
            "Move your Mosquito next to the black Queen.",
            Move {
                piece: "wM",
                to: "/bQ",
            },
            "Click the Mosquito the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["/bQ"])
        .with_arrows(&[("wM", "/bQ")]),
        Step::read(
            "She can't move, and Black can't place on either ringed hex.",
            &["wM-", "-bQ"],
        ),
    ],
    outro: "",
};

const CAVERN: Lesson = Lesson {
    id: "tactics.cavern",
    title: "Caverns",
    intro: &["Some kill spots are caves: walled in, with a gate in front. Walking pieces can't get in, but jumpers and climbers can."],
    brief: "hivegame.com game 211e4288 (2231 vs 1984), White to move at ply 29. bQ's last kill spot (-wG1) is a cave no white walker reaches; White won with wB1 -wG1.",
    setup: Setup::Position("A+sqa+A2+(m+M1-A)3-(p+a1=B1-(GP2=b1-(Qa1=b))),w"),
    steps: &[Step::play(
        "The last kill spot is a cave your Ants can't reach. Your Beetle can: finish the game.",
        Move {
            piece: "wB1",
            to: "-wG1",
        },
        "Click the Beetle the arrow starts from, then the highlighted hex.",
    )
    .highlighted(&["-wG1"])
    .with_arrows(&[("wB1", "-wG1")])],
    outro: "Keep a Beetle, Grasshopper or Ladybug ready for the last kill spot.",
};

const ANTI_SPAWN: Lesson = Lesson {
    id: "tactics.anti_spawn",
    title: "The anti-spawn defence",
    intro: &["Put your own pieces on opposite sides of your Queen. Then every one of her kill spots touches one of them, and Black can never drop a piece next to her."],
    brief: "hivegame.com game 38fd6ffc (2349 vs 1729), White to move at ply 13. White played wA2 wQ-, sandwiching wQ between wP and wA2; Black answered bB2 wQ, but has no drop spot next to her.",
    setup: Setup::Position("AB1-(q-b+Pl4=b3+Q),w"),
    steps: &[
        Step::play(
            "Place an Ant on the side of your Queen opposite your Pillbug.",
            Move {
                piece: "wA2",
                to: "wQ-",
            },
            "Choose an Ant from your reserve, then the highlighted hex.",
        )
        .marked(&["wP"])
        .highlighted(&["wQ-"])
        .reply("bB2", "wQ"),
        Step::read(
            "Their Beetle covered your Queen, but every kill spot touches your Pillbug or Ant: nowhere to drop.",
            &["wP", "wA2"],
        ),
    ],
    outro: "",
};
