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
        SPIDER,
        SPIDER_DEAD_END,
        SPIDER_DOOR,
        SPIDER_POWER,
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
    brief: "Placeholder: choose a simple position where the Spider is not pinned, but backtracking would be needed to complete three steps.",
    setup: Setup::Pending,
    steps: &[Step::read("This position is being prepared. A Spider cannot backtrack during its move.", &[])],
    outro: "",
};

const SPIDER_DOOR: Lesson = Lesson {
    id: "bugs.spider_door",
    title: "The Door",
    intro: &[
        "An open Door can give a Spider different ways to complete its three-step crawl.",
    ],
    brief: "Placeholder: choose a simple Door position that gives the Spider multiple legal three-step routes.",
    setup: Setup::Pending,
    steps: &[Step::read("This position is being prepared. We will explore the routes an open Door gives a Spider.", &[])],
    outro: "",
};

const SPIDER_POWER: Lesson = Lesson {
    id: "bugs.spider_power",
    title: "Spider power",
    intro: &[
        "Two Spiders [[wS1]] [[wS2]] can work together to surround the Black Queen [[bQ]]. Each Spider must crawl exactly three spaces on every move.",
        "Only the White Spiders can move in this exercise. The Queens and blank tiles stay fixed, and Black does not reply.",
    ],
    brief: "Supplied HOP Ap-A1+a1-(q-ga2-(lG+PS!2-(Q+S1-M+a-L))),w. Keep only wQ, bQ, wS1 and wS2 visible. White repeatedly moves either Spider by normal three-step crawls until bQ is surrounded and both Spiders touch her. A five-move solution exists.",
    setup: Setup::BlankExercise {
        position: "Ap-A1+a1-(q-ga2-(lG+PS!2-(Q+S1-M+a-L))),w",
        movable: &["wS1", "wS2"],
        swap: None,
    },
    steps: &[
        Step::play(
            "You play as White. Use both Spiders [[wS1]] [[wS2]] to surround the Black Queen [[bQ]]. You can keep moving either Spider until you win. Blank tiles and Queens stay fixed.",
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
