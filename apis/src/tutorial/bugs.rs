use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};

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
        SPIDER,
        SPIDER_DEAD_END,
        SPIDER_DOOR,
        SPIDER_POWER,
        BEETLE,
        BEETLE_SPAWN,
        BEETLE_GATE,
        GRASSHOPPER,
        GRASSHOPPER_PLACE,
        GRASSHOPPER_PATHS,
        LADYBUG,
        LADYBUG_GATE,
        PILLBUG,
        PILLBUG_SAVE,
        PILLBUG_ESCAPE,
        PILLBUG_ENEMY,
        PILLBUG_GATE,
        PILLBUG_DEACTIVATE,
        MOSQUITO,
        MOSQUITO_CLIMB,
        MOSQUITO_ABILITY,
        MOSQUITO_OTHER,
    ],
};

const QUEEN: Lesson = Lesson {
    id: "bugs.queen",
    title: "Queen Bee",
    intro: &[
        "Keep your Queen [[wQ]] safe: if she is surrounded, you lose.",
        "She moves one hex per turn by crawling.",
        "**Crawling** means sliding along the hive's edge.",
    ],
    brief: "Supplied HOP, White to move. wQ steps to bA2\\, touching the black Ant; Black places bG1 at bL\\; wQ then reaches bA2-, the green hex in line with both black Ants, pinning them both.",
    setup: Setup::Position("A+q+M2-(l+La!-a2-Q),w"),
    steps: &[
        Step::play(
            "You play as White.\nMove your Queen [[wQ]] to reach the [green]green hex [/green]in 2 moves.",
            Move {
                piece: "wQ",
                to: "bA2\\",
            },
            "Choose the shortest route to the [green]green hex[/green].",
        )
        .highlighted(&["bA2-"])
        .hint_after(2)
        .reply("bG1", "bL\\")
        .your_turn(),
        Step::play(
            "Your turn again.\nMove your Queen [[wQ]] into the [green]green hex[/green].",
            Move {
                piece: "wQ",
                to: "bA2-",
            },
            "Move your Queen into the green hex.",
        )
        .highlighted(&["bA2-"])
        .hint_after(2),
        Step::read(
            "Well done! Your Queen [[wQ]] reached the goal and pinned both black Ants [[bA1]]",
            &[],
        )
        .with_completion_tip(
            "Tip: Move your Queen [[wQ]] out of danger when you can.",
        ),
    ],
    outro: "",
};

const FREEDOM_TO_MOVE: Lesson = Lesson {
    id: "bugs.freedom_to_move",
    title: "Freedom to Move Rule",
    intro: &[
        "Crawling pieces must have room to slide.\nYour Ladybug [[wL]] and Ant [[wA2]] block the Queen's [[wQ]] route.",
        "This narrow opening is a **gate**: the Queen [[wQ]] cannot squeeze through it.",
    ],
    brief: "Supplied HOP, White to move. The hex -wL is gated by wL and wA2. Move wA2 anywhere except -wL to open the Gate; Black places bG1 beside bA2 when legal, or elsewhere if White blocks every adjacent placement. Then wQ crawls into -wL.",
    setup: Setup::Position("AB-M+B1-(q-l+La+s!3-(Qa1+ab1-A)),w"),
    steps: &[
        Step::play(
            "You play as White.\nMove your Ant [[wA2]] to open a path for the Queen [[wQ]]",
            MoveExcept {
                piece: "wA2",
                except: "-wL",
            },
            "Move your Ant away from the opening, not into the [green]green hex[/green].",
        )
        .highlighted(&["-wL"])
        .with_red_arrows(&[("wQ", "-wL")])
        .hint_after(2)
        .reply_placement_near("bG1", "bA2")
        .your_turn(),
        Step::play(
            "The path is open. Move your Queen [[wQ]] into the [green]green hex[/green].",
            Move {
                piece: "wQ",
                to: "-wL",
            },
            "Move your Queen into the [green]green hex[/green].",
        )
        .highlighted(&["-wL"])
        .hint_after(2),
        Step::read(
            "Well done! Moving your Ant let the Queen slide through.",
            &[],
        ),
    ],
    outro: "",
};

const DONT_BREAK_THE_HIVE: Lesson = Lesson {
    id: "bugs.dont_break_the_hive",
    title: "Don't break the hive",
    intro: &[
        "A piece must stay connected to the hive throughout its move.",
        "Only your Queen [[wQ]] can move here, the blank tiles stay fixed.\nThe Queen cannot follow the [red]red arrow[/red] because she would lose contact with the hive.",
        "Go around the hive to reach the last empty space next to the black Queen [[bQ]]",
    ],
    brief: "Supplied HOP with fixed blank tiles for every non-Queen piece. Only wQ may move. Crawl around the hive to bQ/, the last hole beside bQ. No opponent moves. The adjacent wM/ hex is red because wQ would lose contact while sliding there directly.",
    setup: Setup::BlankPosition("A+B-aa1-(lqA1-L+A-a2+M)3+m!3-Q,w"),
    steps: &[
        Step::play(
            "You play as White. Keep moving your Queen [[wQ]]\nReach the [green]green hex [/green]to surround the black Queen [[bQ]]",
            Travel { piece: "wQ", to: "bQ/" },
            "Keep your Queen touching the hive as she moves toward the green hex.",
        )
        .highlighted(&["bQ/"])
        .with_initial_red_arrows(&[("wQ", "wM/")])
        .hint_after(2)
        .hint_after_moves("wQ", 6),
        Step::read("White wins!", &[]).announce_winner(),
    ],
    outro: "",
};

const ANT_FIRST_MOVES: Lesson = Lesson {
    id: "bugs.ant_first_moves",
    title: "Ant (first moves)",
    intro: &[
        "The Ant [[wA1]] **crawls** any number of hexes around the edge of the hive in one turn.",
    ],
    brief: "Supplied HOP ALl-qp!1+Q,w. Move the existing wA1 to any legal hex adjacent to bQ.",
    setup: Setup::Position("ALl-qp!1+Q,w"),
    steps: &[
        Step::play(
            "You play as White.\nMove your Ant [[wA1]] beside the black Queen [[bQ]]",
            MovePiece {
                piece: "wA1",
                touching: Some("bQ"),
            },
            "Move your Ant into any hex touching the black Queen.",
        )
        .hint_after(2),
        Step::read(
            "Well done! Your Ant [[wA1]] is beside the black Queen [[bQ]]",
            &[],
        ),
    ],
    outro: "",
};

const ANT: Lesson = Lesson {
    id: "bugs.ant",
    title: "Gate",
    intro: &[
        "A gate can block a crawling piece.",
        "Your Queen [[wQ]] forms one side of this gate.\nMove her away so your Ant [[wA1]] can enter the hole. Blank tiles stay fixed.",
    ],
    brief: "Supplied HOP with wA1 and wA3 swapped without changing the layout. Only wQ and wA1 can move; other non-Queen tiles are blank. wQ moves to -wL to open the Gate, then wA1 reaches wL\\ and wins. No Black reply.",
    setup: Setup::BlankExercise {
        position: "A+PA+b!+lL+Q+A+p+q5-a,w",
        movable: &["wQ", "wA1"],
        swap: Some(("wA1", "wA3")),
        visible: None,
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White. Open the gate.\nUse your Ant [[wA1]] to surround the black Queen [[bQ]]",
            Explore {
                pieces: &["wQ", "wA1"],
                condition: All(&[Surrounded("bQ"), Touching("wA1", "bQ")]),
            },
            "Move your Queen away, then slide your Ant into the hole.",
        )
        .with_red_arrows(&[("wA1", "wL\\")])
        .hint_after_moves("wA1", 2)
        .hint_after(2)
        .your_turn(),
        Step::read("Well done! Your Ant [[wA1]] passed through and surrounded the black Queen [[bQ]]", &[])
            .announce_winner(),
    ],
    outro: "",
};

const ANT_ATTACK: Lesson = Lesson {
    id: "bugs.ant_attack",
    title: "Ant (attack!)",
    intro: &[],
    brief: "Supplied HOP ALlm-q!1+Q,w. Move the existing wA1 next to bQ and pin bQ so she has no legal move on Black's following turn. Exactly /bQ, bQ\\ and bQ- succeed. A failed pin is followed by a legal bQ move when one is available.",
    setup: Setup::Position("ALlm-q!1+Q,w"),
    steps: &[
        Step::play(
            "You play as White.\nMove your Ant [[wA1]] next to the black Queen [[bQ]] and pin her (make sure she can't move).",
            MoveSatisfying {
                piece: "wA1",
                condition: All(&[
                    Touching("wA1", "bQ"),
                    Pinned("bQ"),
                    Stuck("bQ"),
                ]),
            },
            "Move your Ant beside the black Queen where she cannot leave without splitting the hive.",
        )
        .highlight_hint_after_retry(2, &["/bQ", "bQ\\", "bQ-"])
        .reply_on_mistake("bQ"),
        Step::read(
            "Well done! The black Queen [[bQ]] is pinned: moving her would split the hive.",
            &[],
        )
        .outlined_red(&["bQ"])
        .with_completion_tip("Tip: Pinning the opponent's Queen [[bQ]] is a typical way to start an attack."),
    ],
    outro: "",
};

const SPIDER: Lesson = Lesson {
    id: "bugs.spider",
    title: "Spider",
    intro: &[
        "The Spider [[wS1]] crawls **exactly 3 hexes** per move.\nAll 3 steps happen in one turn.",
        "It must move sliding around the hive and cannot revisit a hex during that move (**no backtrack**).",
    ],
    brief: "Supplied HOP A+q-l+L+S1-B3-p!4-Qa+a,w. Any legal wS1 crawl completes the lesson. Exactly two destinations, \\wA1 and \\bA2, become green after two wrong attempts.",
    setup: Setup::Position("A+q-l+L+S1-B3-p!4-Qa+a,w"),
    steps: &[
        Step::play(
            "You play as White. The Spider [[wS1]] must crawl exactly 3 hexes.\nClick it, then choose a destination.",
            MovePiece { piece: "wS1", touching: None },
            "Move your Spider, not another piece.",
        )
        .marked(&["wS1"])
        .with_legal_arrows("wS1")
        .with_movement_paths(hive_lib::Bug::Spider)
        .highlight_hint_after(2, &["\\wA1", "\\bA2"]),
        Step::read(
            "Well done! Restart to try the other destination.",
            &[],
        )
        .with_completion_tip(
            "Tip: A Spider [[wS1]] can threaten to pin two pieces at once.",
        ),
    ],
    outro: "",
};

const SPIDER_DEAD_END: Lesson = Lesson {
    id: "bugs.spider_dead_end",
    title: "Spider (2)",
    intro: &[
        "A Spider [[wS1]] cannot revisit a hex during a move. Here the Spider cannot move, as it would go back to the hex where it is now.",
    ],
    brief: "Supplied HOP G+Aq-m+a4-(pL+QM-A+a!1-(Sl1+a)3-P),w. wS1 is not pinned but has no legal three-step crawl. This is an observation lesson: no move or extra Continue click is required.",
    setup: Setup::Position("G+Aq-m+a4-(pL+QM-A+a!1-(Sl1+a)3-P),w"),
    steps: &[
        Step::read(
            "This Spider [[wS1]] has no 3-step route without **backtracking**.\nThere is no move to play; choose Next lesson.",
            &["wS1"],
        )
        .with_backtracking_arrows("wS1")
        .observation(),
    ],
    outro: "",
};

const SPIDER_DOOR: Lesson = Lesson {
    id: "bugs.spider_door",
    title: "The Door",
    intro: &[
        "A **door** is an opening that gives the Spider [[wS1]] a choice of paths.\nEach move still takes exactly 3 steps.",
    ],
    brief: "Supplied HOP A+P+QM+Llb+a3+S4-m6+p+qA6-g!,w. Only wS1 can move; all pieces except wQ, bQ and wS1 are blank. The shortest route to bA1\\ uses two moves, via wA2\\. Small arrows follow the Spider's current legal options.",
    setup: Setup::BlankExercise {
        position: "A+P+QM+Llb+a3+S4-m6+p+qA6-g!,w",
        movable: &["wS1"],
        swap: None,
        visible: None,
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White. The arrows show the Spider's [[wS1]] options.\nKeep moving it until it reaches the green hex.",
            Travel { piece: "wS1", to: "bA1\\" },
            "Follow a three-step route toward the green hex.",
        )
        .marked(&["wS1"])
        .highlighted(&["bA1\\"])
        .with_legal_arrows("wS1")
        .with_movement_paths(hive_lib::Bug::Spider)
        .with_small_arrows(&[])
        .your_turn(),
        Step::read(
            "Well done! Your Spider [[wS1]] reached the green hex beside the black Queen [[bQ]]",
            &[],
        ),
    ],
    outro: "",
};

const SPIDER_POWER: Lesson = Lesson {
    id: "bugs.spider_power",
    title: "Spider power",
    intro: &[],
    brief: "Supplied HOP Ap-A1+a1-(q-ga2-(lG+PS2-(Q+S1-M+a-L))),w. Keep only wQ, bQ, wS1 and wS2 visible. White repeatedly moves either Spider by normal three-step crawls until bQ is surrounded and both Spiders touch her. A five-move solution exists.",
    setup: Setup::BlankExercise {
        position: "Ap-A1+a1-(q-ga2-(lG+PS2-(Q+S1-M+a-L))),w",
        movable: &["wS1", "wS2"],
        swap: None,
        visible: None,
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White and keep taking turns.\nUse both Spiders [[wS1]] [[wS2]] to surround the black Queen [[bQ]]",
            Explore {
                pieces: &["wS1", "wS2"],
                condition: All(&[
                    Surrounded("bQ"),
                    Touching("wS1", "bQ"),
                    Touching("wS2", "bQ"),
                ]),
            },
            "Move either white Spider toward the black Queen.",
        )
        .route_hint_after_moves(12)
        .with_movement_paths(hive_lib::Bug::Spider)
        .your_turn(),
        Step::read(
            "Well done! Both Spiders [[wS1]] [[wS2]] helped surround the black Queen [[bQ]]",
            &[],
        )
        .announce_winner(),
    ],
    outro: "",
};

const GRASSHOPPER: Lesson = Lesson {
    id: "bugs.grasshopper",
    title: "Jump!",
    intro: &[
        "The Grasshopper [[wG1]] **jumps** over a straight line of pieces.\nIt lands in the first empty hex beyond them.",
        "Gates and stacks do not affect the Grasshopper's jump!",
    ],
    brief: "Adapted from a 2000+ game (corpus game 26, after 11 moves each), with bP replaced by bM. White to move. wG1 jumps into a walled-in hole (-bQ); Black's scripted reply bG1 bM\\; then wG1 jumps out again to -bA2. Highlight and arrow for each jump.",
    setup: Setup::Position("A+am+ql+L+B-a5=B2+b4-M+G6-(Qs1-a),w"),
    steps: &[
        Step::play(
            "You play as White.\nJump your Grasshopper [[wG1]] into the green hex.",
            Move {
                piece: "wG1",
                to: "-bQ",
            },
            "Jump in a straight line over pieces, without crossing an empty hex.",
        )
        .highlighted(&["-bQ"])
        .with_legal_arrows("wG1")
        .normal_moves_only()
        .with_red_arrows(&[("wG1", "bA3\\"), ("wG1", "bA3-")])
        .reply("bG1", "bM\\"),
        Step::play(
            "Now jump back out.\nMove your Grasshopper [[wG1]] to the new green hex.",
            Move {
                piece: "wG1",
                to: "-bA2",
            },
            "Jump in a straight line over pieces, without crossing an empty hex.",
        )
        .highlighted(&["-bA2"])
        .with_arrows(&[("wG1", "-bA2")]),
    ],
    outro: "",
};

const GRASSHOPPER_PLACE: Lesson = Lesson {
    id: "bugs.grasshopper_place",
    title: "Place, jump and win",
    intro: &[
        "Plan a Grasshopper's [[wG3]] jump before placing it.\nA poor placement will not make you reach your destination!",
    ],
    brief: "Supplied HOP A+almPQ+AaGg!4=B4=b1-a-L2+(gpSS2=M2=b1-s2+(BG-A1+g1-q))6-s,w. Swap wG1 and wG2 identities without changing geometry. Place wG3 /wA3 beside wG1 and wA3, Black makes a random legal reply preserving the winning jump, then wG3 \\bQ wins. The trap wG3 wS1/ is refuted by bS1 wG3/; other incorrect attempts get a random legal Black reply.",
    setup: Setup::SwapPieces {
        position: "A+almPQ+AaGg!4=B4=b1-a-L2+(gpSS2=M2=b1-s2+(BG-A1+g1-q))6-s,w",
        first: "wG1",
        second: "wG2",
    },
    steps: &[
        Step::play(
            "You play as White. Black may pin a poorly placed Grasshopper [[wG3]]\nPlace it to win with your next jump.",
            PlaceIn(&["/wA3"]),
            "Find a placement where your last Grasshopper can jump beside the black Queen without being pinned.",
        )
        .punished_when(
            &All(&[Placed("wG3"), Touching("wG3", "wS1"), Touching("wG3", "wS2")]),
            "bS1",
            "wG3/",
        )
        .random_reply_on_mistake()
        .hint_after(2)
        .with_hint_arrows(&[("/wA3", "\\bQ")])
        .outlined_red(&["bQ"])
        .your_turn()
        .random_reply_preserving_win("wG3", "\\bQ"),
        Step::play(
            "Black replied. Your Grasshopper [[wG3]] can now win you the game.\nJump into the green hex.",
            Move { piece: "wG3", to: "\\bQ" },
            "Jump your last Grasshopper into the remaining space beside the black Queen.",
        )
        .marked(&["wG3"])
        .outlined_red(&["bQ"])
        .highlighted(&["\\bQ"])
        .normal_moves_only()
        .hint_after(2),
        Step::read(
            "Well done! You placed safely, then jumped in to surround the black Queen [[bQ]]",
            &[],
        )
        .outlined_red(&["bQ"])
        .announce_winner(),
    ],
    outro: "",
};

const GRASSHOPPER_PATHS: Lesson = Lesson {
    id: "bugs.grasshopper_paths",
    title: "Jumping paths",
    intro: &[
        "Each Grasshopper [[wG2]] [[wG3]] needs an unbroken line of pieces to jump over.\nPlan several jumps to reach the black Queen [[bQ]]",
    ],
    brief: "Supplied HOP A+s+GG1-(aGPQAs3-(g-g+laS2-LAga-S3+Mb!4-(qB2=b1-(p1=B1=m)))),w. Swap wG1 and wG3 identities, retaining the layout. Only wG2 and wG3 move; all tiles except those Grasshoppers and both Queens are blank and fixed. A six-move route fills the two remaining bQ neighbours.",
    setup: Setup::BlankExercise {
        position: "A+s+GG1-(aGPQAs3-(g-g+laS2-LAga-S3+Mb!4-(qB2=b1-(p1=B1=m)))),w",
        movable: &["wG2", "wG3"],
        swap: Some(("wG1", "wG3")),
        visible: None,
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White and keep taking turns.\nUse both Grasshoppers [[wG2]] [[wG3]] to fill the last 2 spaces around Black's Queen [[bQ]]",
            Explore {
                pieces: &["wG2", "wG3"],
                condition: All(&[
                    Surrounded("bQ"),
                    Touching("wG2", "bQ"),
                    Touching("wG3", "bQ"),
                ]),
            },
            "Follow the green arrows, or choose Try again to restart.",
        )
        .highlighted(&["\\bA2", "bB2\\"])
        .guided_exploration(10, 16)
        .your_turn(),
        Step::read(
            "Well done! Both Grasshoppers [[wG2]] [[wG3]] surrounded the black Queen [[bQ]]",
            &[],
        )
        .announce_winner(),
    ],
    outro: "",
};

const BEETLE: Lesson = Lesson {
    id: "bugs.beetle",
    title: "Climbing",
    intro: &[
        "The Beetle [[wB1]] moves one hex per turn.
It can crawl on the ground or climb onto other pieces.",
        "Only the top piece in a stack can move. Covered pieces cannot move or use an ability.",
        "To inspect a stack, right-click on desktop. On mobile, hold it, then slide your finger away.",
    ],
    brief: "Original Climbing layout with bM replaced by bB1. Only Queens and Beetles are visible; only wB1 may move. Explore until wB1 covers bQ, then bB1 covers wB1 on the same stack.",
    setup: Setup::BlankExercise {
        position: "AM-Q+G-pb1-PBl!+q5+a,w",
        movable: &["wB1"],
        swap: None,
        visible: Some(&["wQ", "bQ", "wB1", "bB1"]),
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White.

Move your Beetle [[wB1]] around the hive and cover the black Queen [[bQ]]",
            Explore { pieces: &["wB1"], condition: OnTop("wB1", "bQ") },
            "Keep moving your Beetle toward the black Queen.",
        )
        .highlighted(&["bQ"])
        .normal_moves_only()
        .hint_after(2)
        .route_hint_after_moves(6)
        .your_turn()
        .reply("bB1", "wB1"),
        Step::read(
            "Well done! Black covered your Beetle [[wB1]] with their Beetle [[bB1]]
Only the piece on top can move.",
            &[],
        ),
    ],
    outro: "",
};

const BEETLE_SPAWN: Lesson = Lesson {
    id: "bugs.beetle_spawn",
    title: "New spawn points",
    intro: &[
        "When you consider placement, a stack takes the colour of the topmost piece.\nCovering a black piece with your Beetle [[wB1]] makes that hex count as white.",
        "This can create **spawn points**: new spaces where you may place pieces from the reserves.",
    ],
    brief: "Supplied HOP G+pqg!1=B1-Q3+a,w. Move wB1 from wG1 onto bP, changing that stack to White. Black places bA2 legally, then White places a reserve piece on one of the two newly opened hexes shared by wG1 and wB1.",
    setup: Setup::Position("G+pqg!1=B1-Q3+a,w"),
    steps: &[
        Step::play(
            "You play as White.\nCover the black Pillbug [[bP]] with your Beetle [[wB1]] to create new placement spaces.",
            Move { piece: "wB1", to: "bP" },
            "Move your Beetle onto the black Pillbug.",
        )
        .marked(&["wB1"])
        .hint_after(2)
        .reply_placement("bA2"),
        Step::play(
            "The covered hex now counts as White.\nPlace any piece from the reserve on one of the 2 new spawn points created.",
            PlaceIn(&["\\wG1", "wB1\\"]),
            "Choose a piece from the reserve, then one of the two green hexes.",
        )
        .marked(&["wB1"])
        .highlighted(&["\\wG1", "wB1\\"])
        .without_mark_numbers(),
        Step::read(
            "Well done! Your Beetle [[wB1]] created two new placement spaces.",
            &[],
        ),
    ],
    outro: "",
};

const BEETLE_GATE: Lesson = Lesson {
    id: "bugs.beetle_gate",
    title: "Beetle gates",
    intro: &["Gates exist above the ground too.\nA Beetle [[wB1]] cannot pass between stacks higher than both its starting and landing levels."],
    brief: "hivegame.com/game/OiLpk05USK1S (229b05d2, 1963 vs 1990), after move 40. wB1 sits on bQ, whose last empty kill spot is -bB1, but bB1 (on wG1) and bB2 (on wM) gate the way down. Puzzle: climb onto either black Beetle; blank pieces remain fixed; then wB1 drops into -bB1 and wins. Both routes verified.",
    setup: Setup::BlankExercise {
        position: "A+GA+L+G+plPQ+A+a2=b6=M6=b9=m1-a2+(q1=B)4-Ga9-g,w",
        movable: &["wB1"],
        swap: None,
        visible: Some(&["wB1", "wB2", "bB1", "bB2", "bQ"]),
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "The two black Beetle [[bB1]] stacks block the white beetle [[wB1]] from reaching the green hex (red arrow).\nClimb your Beetle [[wB1]] onto either stack first.",
            Reach(Not(&All(&[
                Not(&OnTop("wB1", "bB1")),
                Not(&OnTop("wB1", "bB2")),
            ]))),
            "Climb onto one of the two outlined black Beetles.",
        )
        .marked(&["bB1", "bB2"])
        .without_mark_numbers()
        .with_red_arrows(&[("wB1", "-bB1")])
        .highlighted(&["-bB1"])
        .your_turn(),
        Step::play(
            "The way down is now open.\nMove your Beetle [[wB1]] into the green hex to win.",
            Move {
                piece: "wB1",
                to: "-bB1",
            },
            "Move your Beetle into the green hex.",
        )
        .highlighted(&["-bB1"]),
        Step::read("Well done! You surrounded the black Queen [[bQ]]", &[]).announce_winner(),
    ],
    outro: "",
};

const MOSQUITO: Lesson = Lesson {
    id: "bugs.mosquito",
    title: "Mosquito",
    intro: &[
        "The Mosquito [[wM]] copies the movement of any piece touching it.\nIts options change as its neighbours change.",
        "Here, your Mosquito [[wM]] touches a Queen [[wQ]], Ladybug [[wL]], Ant [[wA3]] and Grasshopper [[bG1]]",

    ],
    brief: "Supplied HOP. White solo exercise, exactly two wM moves. The four successful first destinations are (18,15), (16,16), (18,16), (18,18); then (15,18) wins. Reveal both Queens, wM and the pieces currently adjacent to wM.",
    setup: Setup::BlankExercise {
        position: "A+aqm+lLM-g-a-b4=B9=b2-S+AG3+s+S+a!6+A6-Q,w",
        movable: &["wM"],
        swap: None,
        visible: Some(&["wQ", "bQ", "wM"]),
        start_as: None,
        extras: &[],
        reveal_neighbours: Some("wM"),
    },
    steps: &[
        Step::play(
            "You play as White. Check your Mosquito's [[wM]] neighbours and move by copying their movements.\nSurround the black Queen [[bQ]] in two moves.",
            Explore {
                pieces: &["wM"],
                condition: Won(hive_lib::Color::White),
            },
            "Find a two-move route. Your Mosquito copies the pieces it touches.",
        )
        .outlined_green(&["wM"])
        .outlined_red(&["bQ"])
        .thin_frames()
        .with_movement_paths(hive_lib::Bug::Ladybug)
        .hint_after(2)
        .retry_after_moves(2)
        .your_turn(),
        Step::read(
            "Well done! Your Mosquito [[wM]] used the ladybug's movement [[wL]] to surround the black Queen [[bQ]]",
            &[],
        )
        .announce_winner(),
    ],
    outro: "",
};

const MOSQUITO_CLIMB: Lesson = Lesson {
    id: "bugs.mosquito_climb",
    title: "The Mosqueetle",
    intro: &[
        "Beside a Beetle [[bB1]], your Mosquito [[wM]] can copy its climbing movement.",
        "Once on top of the hive, a Mosquito [[wM]] must move like a Beetle [[wB1]] until it comes down.",
    ],
    brief: "Supplied HOP. wM pins bB1 at /bB1, bB1\\ or bB1-. Black places bA1 legally near bS1. White climbs onto bB1; Black places bA2 near bS1. White then covers bQ with no further reply. Green pinning destinations appear only after two errors.",
    setup: Setup::Position("AM+L-p+s1+Q4-qb!,w"),
    steps: &[
        Step::play(
            "You play as White. The goal is to cover the black Queen [[bQ]]\nFirst, pin the black Beetle [[bB1]] with your Mosquito [[wM]]",
            MoveSatisfying {
                piece: "wM",
                condition: All(&[Touching("wM", "bB1"), Pinned("bB1")]),
            },
            "Pin the black Beetle from the side away from its Queen, or it can cover your Mosquito.",
        )
        .outlined_red(&["bQ"])
        .thin_frames()
        .normal_moves_only()
        .highlight_hint_after(2, &["/bB1", "bB1\\", "bB1-"])
        .punished_by(&[("bB1", "wM")])
        .reply_placement_near("bA1", "bS1")
        .your_turn(),
        Step::play(
            "Black replied with a placement.\nClimb onto the black Beetle [[bB1]] with your Mosquito [[wM]]",
            Move { piece: "wM", to: "bB1" },
            "Move your Mosquito onto the black Beetle.",
        )
        .outlined_red(&["bQ"])
        .thin_frames()
        .normal_moves_only()
        .hint_after(2)
        .reply_placement_near("bA2", "bS1")
        .your_turn(),
        Step::play(
            "Your Mosquito [[wM]] still moves like a Beetle [[wB1]]\nCover the black Queen [[bQ]]",
            Move { piece: "wM", to: "bQ" },
            "Move your Mosquito from the stack onto the black Queen.",
        )
        .outlined_red(&["bQ"])
        .thin_frames()
        .normal_moves_only()
        .hint_after(2),
        Step::read(
            "Well done! Your Mosquito [[wM]] copied the black Beetle [[bB1]] and covered the black Queen [[bQ]]",
            &[],
        ),
    ],
    outro: "",
};

const MOSQUITO_ABILITY: Lesson = Lesson {
    id: "bugs.mosquito_ability",
    title: "Copy the Pillbug's ability too!",
    intro: &[
        "A Mosquito [[wM]] can copy a neighbouring Pillbug's [[bP]] ability, even an opponent's.",
        "The One Hive Rule still applies.\nA piece cannot be lifted if removing it would split the hive.",

    ],
    brief: "Supplied HOP. The only useful wM approaches are -bP and bP-. Positions bP\\ and /bP pin bP and cannot lift it. After a correct approach, wM may lift bP, or bQ when legal. No Black reply between the two White moves.",
    setup: Setup::Position("AM1+Q1-(Llm!-p2-q),w"),
    steps: &[
        Step::play(
            "You play as White.\nMove your Mosquito [[wM]] beside the black Pillbug [[bP]] without pinning it.",
            MoveSatisfying {
                piece: "wM",
                condition: All(&[Touching("wM", "bP"), Not(&Pinned("bP"))]),
            },
            "Approach from a side that leaves the black Pillbug free to move.",
        )
        .normal_moves_only()
        .explain_when(
            &Pinned("bP"),
            "The black Pillbug is pinned. Lifting it would split the hive and break the One Hive Rule. Try another approach.",
            &["bP"],
        )
        .highlight_hint_after(2, &["-bP", "bP-"])
        .your_turn(),
        Step::play(
            "Your Mosquito [[wM]] can copy the Pillbug's [[bP]] ability.\nSelect the black Pillbug [[bP]] or Queen [[bQ]], then an empty hex beside your Mosquito [[wM]]",
            LiftAnyOf { pieces: &["bP", "bQ"], by: "wM" },
            "Lift an adjacent black piece using the Mosquito's copied ability; do not move the Mosquito itself.",
        )
        .prompt_when(
            &Not(&Touching("wM", "bQ")),
            "Your Mosquito [[wM]] can copy the Pillbug's [[bP]] ability.\nSelect the black Pillbug [[bP]], then an empty hex beside your Mosquito [[wM]]",
        )
        .hint_after(2),
        Step::read(
            "Well done! Your Mosquito [[wM]] copied the Pillbug's [[bP]] ability to lift an enemy piece.",
            &[],
        ),
    ],
    outro: "",
};

const MOSQUITO_OTHER: Lesson = Lesson {
    id: "bugs.mosquito_other",
    title: "Not another Mosquito",
    intro: &[
        "A Mosquito [[wM]] cannot copy another Mosquito [[bM]], even one on top of the hive.",


    ],
    brief: "Supplied HOP. wM pins bB1 from bB1\\ or bB1-. Black replies bM bB1; wM then has no legal move. White moves wA1 next to wM in a way that frees it. Black makes a random legal placement beside bP. Any legal normal wM move completes.",
    setup: Setup::Position("AL+Q+P1+M2-(l-qm!2+b2-p),w"),
    steps: &[
        Step::play(
            "You play as White.\nPin the black Beetle [[bB1]] with your Mosquito [[wM]]",
            MoveSatisfying {
                piece: "wM",
                condition: All(&[Touching("wM", "bB1"), Pinned("bB1")]),
            },
            "Move your Mosquito beside the black Beetle where leaving would split the hive.",
        )
        .normal_moves_only()
        .punish_cover(&["bB1"], &["wM"])
        .explain_when(
            &OnTop("bB1", "wM"),
            "The black Beetle covered your Mosquito. Pin the Beetle before moving beside it.",
            &[],
        )
        .highlight_hint_after(2, &["bB1\\", "bB1-"])
        .reply("bM", "bB1")
        .your_turn(),
        Step::play(
            "Black replied with a Mosquito [[bM]], which yours cannot copy.\nMove your Ant [[wA1]] beside your Mosquito [[wM]] to free it.",
            MoveSatisfying {
                piece: "wA1",
                condition: All(&[Touching("wA1", "wM"), Not(&Stuck("wM"))]),
            },
            "Move your Ant beside the Mosquito without pinning it.",
        )
        .normal_moves_only()
        .punish_cover(&["bB1"], &["wM"])
        .hint_after(2)
        .random_placement_near("bP")
        .your_turn(),
        Step::play(
            "Your Mosquito [[wM]] can now copy the Ant [[wA1]]\nMake any legal move with the Mosquito [[wM]]",
            MovePiece { piece: "wM", touching: None },
            "Move your Mosquito using its Ant neighbour.",
        )
        .normal_moves_only()
        .punish_cover(&["bB1"], &["wM"])
        .hint_after(2),
        Step::read(
            "Well done! Your Ant [[wA1]] freed the Mosquito [[wM]] by giving it a movement to copy.",
            &[],
        ),
    ],
    outro: "",
};

const LADYBUG: Lesson = Lesson {
    id: "bugs.ladybug",
    title: "Ladybug",
    intro: &[
        "The Ladybug [[wL]] moves exactly three hexes: **two on top of the hive, then one down** into an empty hex.",
        "It cannot stop on top or skip either climbing step.",
    ],
    brief: "Corpus game 19, after seven moves each. One example route is wL -> bL -> bM -> bP-, two steps on top and one down.",
    setup: Setup::Position("A+al-L+Q+M+Ps+p+m-q,w"),
    steps: &[Step::play(
        "You play as White. Arrows show a three-step path.\nMove your Ladybug [[wL]] into the hole.",
        Visit { piece: "wL", hexes: &["bP-"] },
        "Take two steps on top of the hive, then one step down.",
    )
    .marked(&["wL"])
    .with_arrows(&[("wL", "bL"), ("bL", "bM"), ("bM", "bP-")])],
    outro: "",
};

const LADYBUG_GATE: Lesson = Lesson {
    id: "bugs.ladybug_gate",
    title: "Beetle gates again",
    intro: &[
        "The Ladybug [[wL]] must obey gates on top of the hive.\nHigh stacks can block its route.",
        "The red arrow shows a blocked climb.\nThe Ladybug [[wL]] cannot reach the three red hexes in one move.",
    ],
    brief: "Supplied HOP. Only wL moves; bQ and the two adjacent Beetle stacks stay visible. The empty hex NW of bS2 is initially unreachable; both other empty neighbours are legal. wL can first land at bQ/ or -bQ, then pin bQ from bQ-, bQ\\ or /bQ. No opponent reply.",
    setup: Setup::BlankExercise {
        position: "A+aps!-lg+Q+Aa-sA5=b5=B5=m5=b7=M7=B3-g5+L+S6-q7-P8-a,w",
        movable: &["wL"],
        swap: None,
        visible: Some(&["wL", "bQ", "wB1", "wB2", "bB1", "bB2", "wM", "bM"]),
        start_as: None,
        extras: &[],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White. The direct route is blocked.\n\nPin the black Queen [[bQ]] with your Ladybug [[wL]] in two moves.",
            MovePiece { piece: "wL", touching: Some("bQ") },
            "Move your Ladybug around the stacks so it can pin the black Queen next turn.",
        )
        .marked(&["wL"])
        .forbidden(&["bQ\\", "/bQ", "bQ-"])
        .with_red_arrows(&[("wL", "bG1")])
        .punish_cover(&["bB1", "bB2"], &["wL"])
        .explain_when(
            &Not(&All(&[Not(&OnTop("bB1", "wL")), Not(&OnTop("bB2", "wL"))])),
            "The black Beetle covered your Ladybug. Keep her out of its reach.",
            &[],
        )
        .hint_after(2)
        .your_turn(),
        Step::play(
            "Your turn again.\n\nMove your Ladybug [[wL]] beside the black Queen [[bQ]] and pin her.",
            MoveSatisfying {
                piece: "wL",
                condition: All(&[Touching("wL", "bQ"), Pinned("bQ"), Stuck("bQ")]),
            },
            "Move your Ladybug beside the black Queen where she cannot leave without splitting the hive.",
        )
        .marked(&["wL"])
        .punish_cover(&["bB1", "bB2"], &["wL"])
        .explain_when(
            &Not(&All(&[Not(&OnTop("bB1", "wL")), Not(&OnTop("bB2", "wL"))])),
            "The black Beetle covered your Ladybug. Keep her out of its reach.",
            &[],
        )
        .hint_after(2),
        Step::read(
            "Well done! Your Ladybug [[wL]] went around the gate and pinned the black Queen [[bQ]]",
            &[],
        )
        .outlined_red(&["bQ"]),
    ],
    outro: "",
};

const PILLBUG: Lesson = Lesson {
    id: "bugs.pillbug",
    title: "The Pillbug's ability",
    intro: &[
        "The Pillbug [[wP]] has two options:",
        "• Crawl one hex, like the Queen [[wQ]]",
        "• Lift an adjacent piece (yours or your opponent's) over its back and then drop it into another empty hex next to itself.",
        "Its special ability has **limits**:",
        "• The hive must stay connected.",
        "• It cannot lift any piece from a stack, even the top one.",
        "• It cannot lift the piece moved on the previous turn.",
        "• A piece moved this way cannot move on the next turn or use a special ability.",
    ],
    brief: "Supplied HOP AB+P-lg!1+Q1-S+G4-(qpm2-a),w. wP lifts wB1 to wP SE (wP\\), beside bQ. The Beetle cannot reach that empty hex in one normal move. Three legal drops are wP\\, \\wP and wP/.",
    setup: Setup::Position("AB+P-lg!1+Q1-S+G4-(qpm2-a),w"),
    steps: &[
        Step::play(
            "You play as White. Use the Pillbug's [[wP]] ability.\nMove your Beetle [[wB1]] into the green hex beside the black Queen [[bQ]]",
            Lift { piece: "wB1", by: "wP", to: "wP\\" },
            "Use your Pillbug to move the Beetle into the green hex beside the black Queen.",
        )
        .marked(&["wP"])
        .highlighted(&["wP\\"])
        .with_arrows(&[
            ("wB1", "wP"), ("wP", "wP\\"),
            ("wP", "\\wP"), ("wP", "wP/"),
        ])
        .with_red_arrows(&[("wB1", "wP\\")]),
        Step::read(
            "Well done! Your Pillbug [[wP]] moved the Beetle [[wB1]] beside the black Queen [[bQ]] in one turn.",
            &[],
        )
        .with_completion_tip("Tip: The Pillbug [[wP]] often allows you to move pieces into spaces that cannot be reached otherwise!"),
    ],
    outro: "",
};

const PILLBUG_SAVE: Lesson = Lesson {
    id: "bugs.pillbug_save",
    title: "Pillbug, save the Queen!",
    intro: &[
        "Your Queen [[wQ]] has only one empty neighbouring hex.",
        "Your Pillbug [[wP]] can move her out of danger.",
    ],
    brief: "Supplied rescue HOP. Complete the lesson when wP moves wQ to any legal drop. Keep the rescued Queen visible on completion.",
    setup: Setup::Position("A+qA+M2+(l+Lg-a+g!2-(QP1-a))2-p,w"),
    steps: &[
        Step::play(
            "You play as White.

Move your Queen [[wQ]] to safety using the Pillbug [[wP]]",
            LiftAny { piece: "wQ", by: "wP" },
            "Select your Queen, then an empty hex beside the Pillbug.",
        )
        .outlined_green(&["wQ"])
        .hint_after(2),
        Step::read("Well done! Your Pillbug [[wP]] moved your Queen [[wQ]] to safety.", &[]),
    ],
    outro: "",
};

const PILLBUG_ESCAPE: Lesson = Lesson {
    id: "bugs.pillbug_escape",
    title: "Make room for the Queen",
    intro: &[
        "A Pillbug [[wP]] can also move itself one hex, like a Queen [[wQ]]",
        "Moving it away creates another empty space beside your Queen [[wQ]]",
    ],
    brief: "Use the same initial rescue position. Complete when wP makes a normal crawl away from wQ; do not use its lifting ability.",
    setup: Setup::Position("A+qA+M2+(l+Lg-a+g!2-(QP1-a))2-p,w"),
    steps: &[
        Step::play(
            "You play as White. Try another way to make space.

Move the Pillbug [[wP]] itself away from your Queen [[wQ]]",
            MoveSatisfying { piece: "wP", condition: Not(&Touching("wP", "wQ")) },
            "Move the Pillbug itself one hex away from your Queen.",
        )
        .outlined_green(&["wQ"])
        .normal_moves_only()
        .hint_after(2),
        Step::read("Well done! Moving your Pillbug [[wP]] gave your Queen [[wQ]] more space.", &[]),
    ],
    outro: "",
};

const PILLBUG_ENEMY: Lesson = Lesson {
    id: "bugs.pillbug_enemy",
    title: "Move enemy pieces",
    intro: &[
        "The Pillbug [[wP]] can lift either player's pieces.",
        "It cannot lift a piece moved on the previous turn.\nThe marked black Ant [[bA2]] has just moved, so it cannot be touched by the Pillbug's ability.",
        "The other black Ant [[bA1]] can still be lifted.",
    ],
    brief: "Supplied HOP with bA1 and bA2 identities swapped without changing geometry. The last-moved, stunned Ant is bA2; only bA1 can be lifted by wP. Any of its three legal drops completes the lesson.",
    setup: Setup::SwapPieces {
        position: "A+qA2+(l+Lg-a!-P-a2+M2-Q)2-p,w",
        first: "bA1",
        second: "bA2",
    },
    steps: &[
        Step::play(
            "You play as White. The marked Ant [[bA2]] cannot be lifted this turn.\nUse your Pillbug [[wP]] to lift Black's other Ant [[bA1]]",
            LiftAny { piece: "bA1", by: "wP" },
            "Use your Pillbug to lift the black Ant that did not move last turn.",
        )
        .previous_move_from("\\wP")
        .hint_after(2),
        Step::read(
            "Well done! Your Pillbug [[wP]] lifted the black Ant [[bA1]] that had not just moved.",
            &[],
        ),
    ],
    outro: "",
};

const PILLBUG_GATE: Lesson = Lesson {
    id: "bugs.pillbug_gate",
    title: "Beetle gates (part 3)",
    intro: &[
        "The highlighted stacks block the Pillbug [[wP]] from lifting your Queen [[wQ]]",
        "Open the gate with a white Beetle [[wB1]]\n\nBe careful: one stack hides a black Beetle [[bB1]] that could cover your Queen [[wQ]]",
    ],
    brief: "Supplied HOP. wB1 can move to any of its four legal destinations except wQ and wP, opening a legal Queen lift. Every wB2 move frees bB1, whose legal reply bB1 wQ covers the Queen. After a correct wB1 move, wP lifts wQ to any legal drop. No Black reply after a correct first move.",
    setup: Setup::Position("A+a-MAL+QaG7=B4+(bP1=B1+A+mG)5-(lbS1-qp-a!)6-s-S,w"),
    steps: &[
        Step::play(
            "You play as White. Keep your Queen [[wQ]] and Pillbug [[wP]] uncovered.\nMove one of your Beetles to remove the gate first.",
            MoveSatisfying {
                piece: "wB1",
                condition: All(&[
                    Not(&OnTop("wB1", "wQ")),
                    Not(&OnTop("wB1", "wP")),
                ]),
            },
            "Move the Beetle that does not free a black Beetle. Leave your Queen and Pillbug uncovered.",
        )
        .marked(&["wB1", "wB2"])
        .without_mark_numbers()
        .with_red_arrows(&[("wQ", "wP")])
        .normal_moves_only()
        .punished_by(&[("bB1", "wQ")])
        .hint_after(2)
        .your_turn(),
        Step::play(
            "The gate is open.\nUse your Pillbug [[wP]] to lift your Queen [[wQ]] to safety.",
            LiftAny { piece: "wQ", by: "wP" },
            "Select your Queen, then an empty hex beside the Pillbug.",
        )
        .hint_after(2),
        Step::read(
            "Well done! You opened the gate safely and rescued your Queen [[wQ]] with the Pillbug [[wP]]",
            &[],
        ),
    ],
    outro: "",
};

const PILLBUG_DEACTIVATE: Lesson = Lesson {
    id: "bugs.pillbug_deactivate",
    title: "Neutralise the Pillbug",
    intro: &[
        "Covering the Pillbug stops its ability.\nCovering the Queen also prevents a rescue.",
        "Here, you will use both Beetles [[wB1]] [[wB2]] to surround the black Queen [[bQ]]\nBlack's Beetle [[bB1]] can cover yours if they get too close, so be careful.",

    ],
    brief: "Supplied HOP, White solo turns. Fixed blank wG2 pins wS1. Any actual White win with the two Beetles succeeds, in any order. Immediately refute any legal Black Beetle cover or Black Queen escape, except after a finished White win.",
    setup: Setup::BlankExercise {
        position: "A+B!1-(apq+A1-a-G2+S3-(l+LBg1-b2-(Q+P1-a))),b",
        movable: &["wB1", "wB2"],
        swap: None,
        visible: Some(&["wQ", "bQ", "wB1", "wB2", "bB1", "bB2", "wP", "bP"]),
        start_as: Some(hive_lib::Color::White),
        extras: &[("wG2", "wS1/")],
        reveal_neighbours: None,
    },
    steps: &[
        Step::play(
            "You play as White and keep taking turns.\nSurround the black Queen [[bQ]] with both Beetles [[wB1]] [[wB2]], preventing a rescue or counterattack.",
            Explore {
                pieces: &["wB1", "wB2"],
                condition: Won(hive_lib::Color::White),
            },
            "Black found a defence. Keep its Queen trapped and your Beetles out of reach of the black Beetle.",
        )
        .highlighted(&["-bQ", "bQ\\"])
        .punish_escape("bQ")
        .punish_cover(&["bB1", "bB2"], &["wB1", "wB2"])
        .hint_after(2)
        .your_turn(),
        Step::read(
            "Well done! Your Beetles [[wB1]] [[wB2]] stopped the rescue and surrounded the black Queen [[bQ]]",
            &[],
        )
        .announce_winner(),
    ],
    outro: "",
};
