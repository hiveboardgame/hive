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
        PILLBUG_ENEMY,
        PILLBUG_WALK,
        PILLBUG_GATE,
        PILLBUG_DEACTIVATE,
        MOSQUITO,
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
            "Blank tiles cannot move. You can keep moving until the lesson is completed. You play as White. Move your Queen [[wQ]] around the hive to reach the green hex and surround the Black Queen [[bQ]].",
            Travel { piece: "wQ", to: "bQ/" },
            "Keep your Queen touching the hive and aim for the green hex.",
        )
        .highlighted(&["bQ/"])
        .forbidden(&["wM/"])
        .hint_after(2),
        Step::read("Well done!", &[]).forbidden(&["wM/"]).announce_winner(),
    ],
    outro: "",
};

const ANT_FIRST_MOVES: Lesson = Lesson {
    id: "bugs.ant_first_moves",
    title: "Ant (first moves)",
    intro: &[
        "The Ant [[wA1]] can crawl around the edge of the hive, as many spaces as you want in a single turn.",
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
    title: "Gate",
    intro: &[
        "A Gate can stop a crawling piece from sliding into an empty hex.",
        "Here, your Queen [[wQ]] forms one side of the Gate. Move her away, then slide your Ant [[wA1]] into the last space around the Black Queen [[bQ]]. Blank tiles stay fixed.",
    ],
    brief: "Supplied HOP with wA1 and wA3 swapped without changing the layout. Only wQ and wA1 can move; other non-Queen tiles are blank. wQ moves to -wL to open the Gate, then wA1 reaches wL\\ and wins. No Black reply.",
    setup: Setup::BlankExercise {
        position: "A+PA+b!+lL+Q+A+p+q5-a,w",
        movable: &["wQ", "wA1"],
        swap: Some(("wA1", "wA3")),
        visible: None,
    },
    steps: &[
        Step::play(
            "You play as White. Surround the Black Queen [[bQ]] using your Ant [[wA1]], removing the Gate first. You may move your Ant around to explore.",
            Explore {
                pieces: &["wQ", "wA1"],
                condition: All(&[Surrounded("bQ"), Touching("wA1", "bQ")]),
            },
            "Open the Gate with your Queen, then move your Ant into the hole.",
        )
        .with_red_arrows(&[("wA1", "wL\\")])
        .hint_after_moves("wA1", 2)
        .hint_after(2)
        .your_turn(),
        Step::read("Well done! Your Ant [[wA1]] passed through the open Gate and surrounded the Black Queen [[bQ]].", &[])
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
            "Let's use our Ant [[wA1]] to start an attack. Move your Ant [[wA1]] near the opponent's Queen [[bQ]] and make sure she can't move.",
            MoveSatisfying {
                piece: "wA1",
                condition: All(&[
                    Touching("wA1", "bQ"),
                    Pinned("bQ"),
                    Stuck("bQ"),
                ]),
            },
            "Move your Ant next to the Black Queen so she cannot move without breaking the hive.",
        )
        .highlight_hint_after(2, &["/bQ", "bQ\\", "bQ-"])
        .reply_on_mistake("bQ"),
        Step::read(
            "Well done! The Black Queen [[bQ]] cannot escape now, and we say she is **pinned** (she cannot move without breaking the hive!).",
            &[],
        )
        .outlined_red(&["bQ"])
        .with_completion_tip("Tip: An attack often starts by pinning the opponent's Queen!"),
    ],
    outro: "",
};

const SPIDER: Lesson = Lesson {
    id: "bugs.spider",
    title: "Spider",
    intro: &[
        "The Spider [[wS1]] crawls exactly three spaces around the edge of the hive in one move. It cannot stop early or take extra steps.",
        "It cannot revisit a space during the same move, and it must keep sliding in contact with the hive.",
    ],
    brief: "Supplied HOP A+q-l+L+S1-B3-p!4-Qa+a,w. Any legal wS1 crawl completes the lesson. Exactly two destinations, \\wA1 and \\bA2, become green after two wrong attempts.",
    setup: Setup::Position("A+q-l+L+S1-B3-p!4-Qa+a,w"),
    steps: &[
        Step::play(
            "You play as White. Click your Spider [[wS1]] to see where it can go. It must move exactly 3 spaces around the hive, no more, no less.",
            MovePiece { piece: "wS1", touching: None },
            "Move your Spider. It must crawl exactly 3 spaces around the hive.",
        )
        .marked(&["wS1"])
        .highlight_hint_after(2, &["\\wA1", "\\bA2"]),
        Step::read(
            "Well done! Feel free to restart the lesson to try a different move.",
            &[],
        )
        .with_completion_tip(
            "Tip: Place your Spider so it threatens to pin two opponent pieces at the same time!",
        ),
    ],
    outro: "",
};

const SPIDER_DEAD_END: Lesson = Lesson {
    id: "bugs.spider_dead_end",
    title: "Spiders cannot backtrack",
    intro: &[
        "A Spider cannot revisit a space during the same move. If its only way to continue would take it back where it came from, it cannot complete its three steps.",
    ],
    brief: "Supplied HOP G+Aq-m+a4-(pL+QM-A+a!1-(Sl1+a)3-P),w. wS1 is not pinned but has no legal three-step crawl. This is an observation lesson: no move or extra Continue click is required.",
    setup: Setup::Position("G+Aq-m+a4-(pL+QM-A+a!1-(Sl1+a)3-P),w"),
    steps: &[
        Step::read(
            "The highlighted Spider [[wS1]] is not pinned, but it cannot complete 3 steps without backtracking. A Spider cannot return to a space it already visited during the same move. There is nothing to move here: continue to the next lesson.",
            &["wS1"],
        )
        .observation(),
    ],
    outro: "",
};

const SPIDER_DOOR: Lesson = Lesson {
    id: "bugs.spider_door",
    title: "The Door",
    intro: &[
        "An open Door gives a Spider different routes around the hive. Every legal move still follows exactly 3 steps without backtracking.",
    ],
    brief: "Supplied HOP A+P+QM+Llb+a3+S4-m6+p+qA6-g!,w. Only wS1 can move; all pieces except wQ, bQ and wS1 are blank. The shortest route to bA1\\ uses two moves, via wA2\\. Small arrows follow the Spider's current legal options.",
    setup: Setup::BlankExercise {
        position: "A+P+QM+Llb+a3+S4-m6+p+qA6-g!,w",
        movable: &["wS1"],
        swap: None,
        visible: None,
    },
    steps: &[
        Step::play(
            "You play as White. Use the Door to move your Spider [[wS1]] into the green hex beside the Black Queen [[bQ]]. Keep moving until you reach it: each Spider move must crawl exactly 3 spaces. The arrows show your current options.",
            Travel { piece: "wS1", to: "bA1\\" },
            "Use the Door to work toward the green hex. Each Spider move must crawl exactly 3 spaces.",
        )
        .marked(&["wS1"])
        .highlighted(&["bA1\\"])
        .with_legal_arrows("wS1")
        .with_small_arrows(&[])
        .your_turn(),
        Step::read(
            "Well done! Your Spider [[wS1]] used the Door to reach the space beside the Black Queen [[bQ]].",
            &[],
        ),
    ],
    outro: "",
};

const SPIDER_POWER: Lesson = Lesson {
    id: "bugs.spider_power",
    title: "Spider power",
    intro: &[],
    brief: "Supplied HOP Ap-A1+a1-(q-ga2-(lG+PS!2-(Q+S1-M+a-L))),w. Keep only wQ, bQ, wS1 and wS2 visible. White repeatedly moves either Spider by normal three-step crawls until bQ is surrounded and both Spiders touch her. A five-move solution exists.",
    setup: Setup::BlankExercise {
        position: "Ap-A1+a1-(q-ga2-(lG+PS!2-(Q+S1-M+a-L))),w",
        movable: &["wS1", "wS2"],
        swap: None,
        visible: None,
    },
    steps: &[
        Step::play(
            "You play as White. Use both Spiders [[wS1]] [[wS2]] to surround the Black Queen [[bQ]]. You can keep moving either Spider until you win.",
            Explore {
                pieces: &["wS1", "wS2"],
                condition: All(&[
                    Surrounded("bQ"),
                    Touching("wS1", "bQ"),
                    Touching("wS2", "bQ"),
                ]),
            },
            "Move one of your Spiders around the hive toward the Black Queen.",
        )
        .your_turn(),
        Step::read(
            "Well done! Both Spiders [[wS1]] [[wS2]] helped surround the Black Queen [[bQ]].",
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
        "The Grasshopper jumps in a straight line over one or more pieces and lands in the first empty hex behind them. It can't jump over an empty hex.",
        "Because it jumps instead of sliding, the Grasshopper is not affected by gates or by stacks!",
    ],
    brief: "Adapted from a 2000+ game (corpus game 26, after 11 moves each), with bP replaced by bM. White to move. wG1 jumps into a walled-in hole (-bQ); Black's scripted reply bG1 bM\\; then wG1 jumps out again to -bA2. Highlight and arrow for each jump.",
    setup: Setup::Position("A+am+ql+L+B-a5=B2+b4-M+G6-(Qs1-a),w"),
    steps: &[
        Step::play(
            "You play as White. Jump your Grasshopper [[wG1]] into the highlighted hole.",
            Move {
                piece: "wG1",
                to: "-bQ",
            },
            "The Grasshopper moves in straight lines only, over pieces, never over gaps.",
        )
        .highlighted(&["-bQ"])
        .with_legal_arrows("wG1")
        .normal_moves_only()
        .with_red_arrows(&[("wG1", "bA3\\"), ("wG1", "bA3-")])
        .reply("bG1", "bM\\"),
        Step::play(
            "No walking piece could have got in there. Now jump your Grasshopper [[wG1]] out again, to the highlighted hex.",
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

const GRASSHOPPER_PLACE: Lesson = Lesson {
    id: "bugs.grasshopper_place",
    title: "Place, jump and win",
    intro: &[
        "Plan your Grasshopper's jumping path before placing it. A tempting placement can leave it pinned before it gets a chance to jump.",
    ],
    brief: "Supplied HOP A+almPQ+AaGg!4=B4=b1-a-L2+(gpSS2=M2=b1-s2+(BG-A1+g1-q))6-s,w. Swap wG1 and wG2 identities without changing geometry. Place wG3 /wA3 beside wG1 and wA3, Black makes a random legal reply preserving the winning jump, then wG3 \\bQ wins. The trap wG3 wS1/ is refuted by bS1 wG3/; other incorrect attempts get a random legal Black reply.",
    setup: Setup::SwapPieces {
        position: "A+almPQ+AaGg!4=B4=b1-a-L2+(gpSS2=M2=b1-s2+(BG-A1+g1-q))6-s,w",
        first: "wG1",
        second: "wG2",
    },
    steps: &[
        Step::play(
            "You play as White. Place your last Grasshopper [[wG3]] so it can jump into the last empty space around the Black Queen [[bQ]] on your next turn. Be careful: Black can pin it if you choose the wrong placement.",
            PlaceIn(&["/wA3"]),
            "Find the right placement for the last Grasshopper to attack the Black Queen!",
        )
        .punished_when(
            &All(&[Placed("wG3"), Touching("wG3", "wS1"), Touching("wG3", "wS2")]),
            "bS1",
            "wG3/",
        )
        .random_reply_on_mistake()
        .highlight_hint_after(2, &["/wA3"])
        .outlined_red(&["bQ"])
        .your_turn()
        .random_reply_preserving_win("wG3", "\\bQ"),
        Step::play(
            "Black replied. Your Grasshopper [[wG3]] has a clear jumping path: jump into the green hex to surround the Black Queen [[bQ]] and win.",
            Move { piece: "wG3", to: "\\bQ" },
            "Jump your last Grasshopper into the final empty space beside the Black Queen.",
        )
        .marked(&["wG3"])
        .outlined_red(&["bQ"])
        .highlighted(&["\\bQ"])
        .normal_moves_only()
        .hint_after(2),
        Step::read(
            "Well done! You chose a safe placement, then jumped in to surround the Black Queen [[bQ]].",
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
        "A Grasshopper needs an unbroken line of pieces to jump over. Use the hive to build a route for both Grasshoppers toward the Black Queen.",
    ],
    brief: "Supplied HOP A+s+GG1-(aGPQAs3-(g-g+laS2-LAga-S3+Mb!4-(qB2=b1-(p1=B1=m)))),w. Swap wG1 and wG3 identities, retaining the layout. Only wG2 and wG3 move; all tiles except those Grasshoppers and both Queens are blank and fixed. A six-move route fills the two remaining bQ neighbours.",
    setup: Setup::BlankExercise {
        position: "A+s+GG1-(aGPQAs3-(g-g+laS2-LAga-S3+Mb!4-(qB2=b1-(p1=B1=m)))),w",
        movable: &["wG2", "wG3"],
        swap: Some(("wG1", "wG3")),
        visible: None,
    },
    steps: &[
        Step::play(
            "You play as White. Keep jumping with your two Grasshoppers [[wG2]] [[wG3]] until they fill the last two spaces around the Black Queen [[bQ]]. Only your Grasshoppers can move; the blank tiles stay fixed.",
            Explore {
                pieces: &["wG2", "wG3"],
                condition: All(&[
                    Surrounded("bQ"),
                    Touching("wG2", "bQ"),
                    Touching("wG3", "bQ"),
                ]),
            },
            "Follow the jumping paths, or try again from the start.",
        )
        .highlighted(&["\\bA2", "bB2\\"])
        .guided_exploration(10, 16)
        .your_turn(),
        Step::read(
            "Well done! Both Grasshoppers [[wG2]] [[wG3]] filled the last spaces around the Black Queen [[bQ]]. White wins!",
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
        "The Beetle [[wB1]] moves one hex per turn. It can crawl on the ground or climb onto another piece, then move across the top of the hive.",
        "There is no height limit for a stack. Only the piece on top can move; every piece covered underneath is immobile until it is uncovered.",
        "A Beetle on the ground cannot squeeze through a narrow Gate. Climbing gives it a way around.",
    ],
    brief: "Supplied HOP AM-Q+G-pm1-PBl!+q5+a,w. wB1 climbs onto bL; Black places bB1 bQ\\ between bQ and bM; wB1 covers bQ; bB1 covers wB1. Show all legal Beetle moves and the illegal Queen gap first, replacing these with a single correct hint after two errors.",
    setup: Setup::Position("AM-Q+G-pm1-PBl!+q5+a,w"),
    steps: &[
        Step::play(
            "You play as White. Climb onto the Black Ladybug [[bL]] with your Beetle [[wB1]]. Green arrows show legal moves; the red arrow points through a Gate your Beetle cannot enter from the ground because of the Freedom to Move Rule.",
            Move { piece: "wB1", to: "bL" },
            "Climb your Beetle onto the Black Ladybug.",
        )
        .marked(&["wB1"])
        .with_legal_arrows("wB1")
        .normal_moves_only()
        .with_red_arrows(&[("wB1", "-bP")])
        .hint_after(2)
        .your_turn()
        .reply("bB1", "bQ\\"),
        Step::play(
            "Black replied by placing a Beetle [[bB1]]. You can look under a stack by right-clicking it on desktop, or by holding it and then sliding your finger elsewhere on mobile. Now move your Beetle [[wB1]] onto the Black Queen [[bQ]].",
            Move { piece: "wB1", to: "bQ" },
            "Move your Beetle onto the Black Queen.",
        )
        .marked(&["wB1"])
        .with_legal_arrows("wB1")
        .normal_moves_only()
        .hint_after(2)
        .reply("bB1", "wB1"),
        Step::read(
            "Well done! Black replied by climbing onto your Beetle [[wB1]]. The Black Queen [[bQ]] and your Beetle are both covered and cannot move. Only the Black Beetle [[bB1]] on top can move.",
            &["bB1"],
        )
        .without_mark_numbers(),
    ],
    outro: "",
};

const BEETLE_SPAWN: Lesson = Lesson {
    id: "bugs.beetle_spawn",
    title: "Spawn points",
    intro: &[
        "For placing new pieces, a stack takes the colour of its top piece. A White Beetle [[wB1]] on top of a Black piece makes that hex count as White.",
        "By covering an opponent's piece, your Beetle can create new spawn points: places where pieces from your reserve can enter the game.",
    ],
    brief: "Supplied HOP G+pqg!1=B1-Q3+a,w. Move wB1 from wG1 onto bP, changing that stack to White. Black places bA2 legally, then White places a reserve piece on one of the two newly opened hexes shared by wG1 and wB1.",
    setup: Setup::Position("G+pqg!1=B1-Q3+a,w"),
    steps: &[
        Step::play(
            "You play as White. Move your Beetle [[wB1]] one space forward onto the Black Pillbug [[bP]] to create new spawn points.",
            Move { piece: "wB1", to: "bP" },
            "Move your Beetle onto the Black Pillbug.",
        )
        .marked(&["wB1"])
        .hint_after(2)
        .reply_placement("bA2"),
        Step::play(
            "Black replied by placing a piece. Your Beetle [[wB1]] makes the covered Pillbug's hex count as White, opening new places to enter the game. Pick any piece from your reserve and place it on one of the two green hexes shared by your Grasshopper [[wG1]] and Beetle.",
            PlaceIn(&["\\wG1", "wB1\\"]),
            "Pick any piece from your reserve, then choose one of the two green spawn points.",
        )
        .marked(&["wB1"])
        .highlighted(&["\\wG1", "wB1\\"])
        .without_mark_numbers(),
        Step::read(
            "Well done! Your Beetle [[wB1]] changed the colour of the stack and created new spawn points for your pieces.",
            &[],
        ),
    ],
    outro: "",
};

const BEETLE_GATE: Lesson = Lesson {
    id: "bugs.beetle_gate",
    title: "Beetle gates",
    intro: &["Gates exist on top of the hive too. A Beetle can't squeeze between two stacks that are higher than both where it starts and where it lands."],
    brief: "hivegame.com/game/OiLpk05USK1S (229b05d2, 1963 vs 1990), after move 40. wB1 sits on bQ, whose last empty kill spot is -bB1, but bB1 (on wG1) and bB2 (on wM) gate the way down. Puzzle: climb onto either black Beetle; blank pieces remain fixed; then wB1 drops into -bB1 and wins. Both routes verified.",
    setup: Setup::BlankExercise {
        position: "A+GA+L+G+plPQ+A+a2=b6=M6=b9=m1-a2+(q1=B)4-Ga9-g,w",
        movable: &["wB1"],
        swap: None,
        visible: Some(&["wB1", "wB2", "bB1", "bB2"]),
    },
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
        .without_mark_numbers()
        .with_red_arrows(&[("wB1", "-bB1")])
        .highlighted(&["-bB1"])
        .your_turn(),
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
    outro: "",
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
        "The Ladybug [[wL]] always moves exactly three hexes: two steps on top of the hive, then one step down into an empty hex.",
        "It can't stop on top, and it can't skip the climb. It's quick and flexible over short distances and can land inside pockets walking pieces can't reach.",
    ],
    brief: "Corpus game 19, after seven moves each. One example route is wL -> bL -> bM -> bP-, two steps on top and one down.",
    setup: Setup::Position("A+al-L+Q+M+Ps+p+m-q,w"),
    steps: &[Step::play(
        "You play as White. Move your Ladybug [[wL]] into the walled-in pocket: up, along the top, and down. The arrows show one three-step path.",
        Visit { piece: "wL", hexes: &["bP-"] },
        "Two steps on top of the hive, then one step down.",
    )
    .marked(&["wL"])
    .with_arrows(&[("wL", "bL"), ("bL", "bM"), ("bM", "bP-")])],
    outro: "No walking piece could ever get in there. Ladybugs are at their best in crowded positions.",
};

const LADYBUG_GATE: Lesson = Lesson {
    id: "bugs.ladybug_gate",
    title: "Beetle gates again",
    intro: &[
        "The Ladybug [[wL]] must obey Gates on top of the hive too. She cannot squeeze between stacks that are too high.",
        "The red hex beside Black's Spider is unreachable from this starting position. The red arrow shows a blocked climb between the two Beetle stacks. Find a different route to pin the Black Queen [[bQ]] in two moves.",
    ],
    brief: "Supplied HOP. Only wL moves; bQ and the two adjacent Beetle stacks stay visible. The empty hex NW of bS2 is initially unreachable; both other empty neighbours are legal. wL can first land at bQ/ or -bQ, then pin bQ from bQ-, bQ\\ or /bQ. No opponent reply.",
    setup: Setup::BlankExercise {
        position: "A+aps!-lg+Q+Aa-sA5=b5=B5=m5=b7=M7=B3-g5+L+S6-q7-P8-a,w",
        movable: &["wL"],
        swap: None,
        visible: Some(&["wL", "bQ", "wB1", "wB2", "bB1", "bB2", "wM", "bM"]),
    },
    steps: &[
        Step::play(
            "You play as White. Find a legal route for your Ladybug [[wL]] to pin the Black Queen [[bQ]] in two moves. The direct route is blocked by Beetle Gates.",
            MovePiece { piece: "wL", touching: Some("bQ") },
            "Move your Ladybug around the Beetle Gates so she can reach the Black Queen on the next move.",
        )
        .marked(&["wL"])
        .forbidden(&["\\bS2"])
        .with_red_arrows(&[("wL", "bG1")])
        .hint_after(2)
        .your_turn(),
        Step::play(
            "Your turn. Move your Ladybug [[wL]] next to the Black Queen [[bQ]] so she cannot move without breaking the hive.",
            MoveSatisfying {
                piece: "wL",
                condition: All(&[Touching("wL", "bQ"), Pinned("bQ"), Stuck("bQ")]),
            },
            "Move your Ladybug next to the Black Queen and pin her.",
        )
        .marked(&["wL"])
        .hint_after(2),
        Step::read(
            "Well done! Your Ladybug [[wL]] went around the Beetle Gates and pinned the Black Queen [[bQ]].",
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
        "The Pillbug [[wP]] is very special. It can move or use its unique ability:",
        "• When it moves, it crawls one hex at a time around the edge of the hive, like the Queen [[wQ]].",
        "• When using its ability, it can pick up any piece next to it, yours or your opponent's, carry it over its back, and put it down in another empty hex next to it.",
        "There are some limitations:",
        "• It can't break the hive in the process!",
        "• It can't move any piece that is part of a stack of Beetles.",
        "• It can't move a piece that moved on the very last turn.",
        "• A piece moved by the Pillbug is also stunned for the turn right after.",
        "We'll go through all of these now.",
    ],
    brief: "Supplied HOP AB+P-lg!1+Q1-S+G4-(qpm2-a),w. wP lifts wB1 to wP SE (wP\\), beside bQ. The Beetle cannot reach that empty hex in one normal move. Three legal drops are wP\\, \\wP and wP/.",
    setup: Setup::Position("AB+P-lg!1+Q1-S+G4-(qpm2-a),w"),
    steps: &[
        Step::play(
            "You play as White. Move your Beetle [[wB1]] using the Pillbug [[wP]] to attack the enemy Queen [[bQ]] in one move. Select the Beetle, then the green hex beside the Black Queen.",
            Lift { piece: "wB1", by: "wP", to: "wP\\" },
            "Use your Pillbug to lift the Beetle into the empty hex next to the Black Queen.",
        )
        .marked(&["wP"])
        .highlighted(&["wP\\"])
        .with_arrows(&[
            ("wB1", "wP"), ("wP", "wP\\"),
            ("wP", "\\wP"), ("wP", "wP/"),
        ])
        .with_red_arrows(&[("wB1", "wP\\")]),
        Step::read(
            "Well done! Your Pillbug [[wP]] carried the Beetle [[wB1]] into an attacking position beside the Black Queen [[bQ]], all in one turn.",
            &[],
        ),
    ],
    outro: "",
};

const PILLBUG_SAVE: Lesson = Lesson {
    id: "bugs.pillbug_save",
    title: "Pillbug, save the Queen!",
    intro: &[],
    brief: "Placeholder: rescue a nearly surrounded Queen using the Pillbug's ability.",
    setup: Setup::Pending,
    steps: &[Step::read("A Queen rescue exercise will be added here.", &[])],
    outro: "",
};

const PILLBUG_ENEMY: Lesson = Lesson {
    id: "bugs.pillbug_enemy",
    title: "Move enemy pieces",
    intro: &[],
    brief: "Placeholder: lift an opponent's piece to defend.",
    setup: Setup::Pending,
    steps: &[Step::read("An exercise about moving enemy pieces will be added here.", &[])],
    outro: "",
};

const PILLBUG_WALK: Lesson = Lesson {
    id: "bugs.pillbug_walk",
    title: "Don't forget it can move!",
    intro: &[],
    brief: "Placeholder: crawl one hex with the Pillbug rather than using its ability.",
    setup: Setup::Pending,
    steps: &[Step::read("A Pillbug movement exercise will be added here.", &[])],
    outro: "",
};

const PILLBUG_GATE: Lesson = Lesson {
    id: "bugs.pillbug_gate",
    title: "Beetle gates (part 3)",
    intro: &[],
    brief: "Placeholder: Beetle Gates prevent the Pillbug from lifting certain pieces.",
    setup: Setup::Pending,
    steps: &[Step::read("A Pillbug Gate exercise will be added here.", &[])],
    outro: "",
};

const PILLBUG_DEACTIVATE: Lesson = Lesson {
    id: "bugs.pillbug_deactivate",
    title: "Deactivate the Pillbug",
    intro: &[],
    brief: "Placeholder: play against a Pillbug by covering it or filling its adjacent spaces.",
    setup: Setup::Pending,
    steps: &[Step::read("An exercise about covering and surrounding a Pillbug will be added here.", &[])],
    outro: "",
};
