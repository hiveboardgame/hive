use super::{
    goal::Goal::*,
    lesson::{Chapter, GuidePage, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "online",
    title: "Playing online",
    summary: "Find a game, use the clock and controls, and make yourself at home.",
    lessons: &[
        INTERFACE,
        CREATE_JOIN,
        TIME_CONTROLS,
        CONTROLS,
        SETTINGS,
        CORRESPONDENCE,
        TOURNAMENTS,
        READY,
    ],
};

const INTERFACE: Lesson = Lesson {
    id: "online.interface",
    title: "Using the play interface",
    intro: &[
        "Click a piece in your reserve to see legal placements, or a piece in play to see its legal moves. The site does not allow illegal moves.",
        "To look under a Beetle [[wB1]] stack, right-click on desktop. On mobile, hold the stack, then slide your finger away.",
        "To use the Pillbug's [[wP]] ability, click the piece you want to move, then its destination. You do not need to select the Pillbug [[wP]] first.",
    ],
    brief: "Supplied HOP GPM1+(QL+lm-p+s!1-A3+a3-q),w. Show the real board, reserves and controls. Any legal White turn, including a Pillbug move of an enemy piece, completes the lesson.",
    setup: Setup::Position("GPM1+(QL+lm-p+s!1-A3+a3-q),w"),
    steps: &[
        Step::play(
            "You play as White. Click a piece in play or in your reserve to see its legal options.\nMake any legal move to try the interface.",
            PlayAny,
            "Click a piece, then one of its highlighted hexes.",
        ),
        Step::read("Well done! You are ready to use the play interface.", &[]),
    ],
    outro: "",
};

const CREATE_JOIN: Lesson = Lesson {
    id: "online.create_join",
    title: "A bit more about hivegame.com",
    intro: &["The homepage lets you create a game or join one.\nThis is a read-only tour; use Continue to follow along."],
    brief: "Actual homepage, rendered read-only. Point to quick games, Custom, and open challenges in three read-only stages.",
    setup: Setup::Guide(GuidePage::Home),
    steps: &[
        Step::read("These hexagonal buttons create new open games with preset time controls.\nPress Continue to see the Custom options.", &[]),
        Step::read("Custom lets you choose your game options.\nPress Continue to see open games.", &[]),
        Step::read("Join accepts a game another player created.\nCheck its time control before joining.", &[]),
    ],
    outro: "Tip: Start with a slower time control (10+10 or 20+20) so you have more time to think.",
};

const TIME_CONTROLS: Lesson = Lesson {
    id: "online.time_controls",
    title: "Reading a time control",
    intro: &[
        "A clock such as **5+4** gives each player 5 minutes, plus 4 seconds after each move.",
        "Here, **1+0** means one minute each, with no extra seconds.\nYour clock runs only on your turn. Reaching zero loses the game.",
    ],
    brief: "A local 1+0 demonstration using the existing four-ply board. Any legal White turn starts Black's clock. Black makes no move and loses at zero; a White timeout requires retry.",
    setup: Setup::Clocked {
        moves: "wL;bL wL-;wM -wL;bM bL-",
        base_minutes: 1,
        increment_seconds: 0,
    },
    steps: &[
        Step::play(
            "You play as White. You have one minute.\nMake any legal placement before your clock reaches zero.",
            PlaceAny,
            "Make a legal White move before your clock reaches zero.",
        ).await_opponent(),
        Step::play(
            "Your clock has stopped. Black will not move.\nYou can watch Black's clock count down to zero if you like (it will turn red).",
            WaitForTimeout(Color::Black),
            "Wait for Black's clock to reach zero.",
        ).await_opponent(),
        Step::read("Black ran out of time. White wins!\nThe losing clock turns red.", &[]).announce_winner(),
    ],
    outro: "",
};

const CONTROLS: Lesson = Lesson {
    id: "online.controls",
    title: "Undo, offer a draw and resign",
    intro: &["These controls sit beside the board.\nIn a game, click once to select an action and again to confirm.\nThese tutorial controls do not affect a real game."],
    brief: "The actual board, reserves and game-control components, with each control highlighted in its normal sidebar position.",
    setup: Setup::Position("L+Q1-l-q!,w"),
    steps: &[
        Step::read("Undo asks to take back your last move.\nYour opponent must agree.", &[]),
        Step::read("Offer draw asks to end the game as a draw.\nIf your opponent declines, play continues.", &[]),
        Step::read("Resign ends the game and gives your opponent the win.", &[]),
    ],
    outro: "",
};

const SETTINGS: Lesson = Lesson {
    id: "online.settings",
    title: "Change settings",
    intro: &["Open your profile menu at the top right, then choose **Config**."],
    brief: "The actual Settings component is rendered read-only in a script-free frame.",
    setup: Setup::Guide(GuidePage::Settings),
    steps: &[
        Step::read("Settings controls board appearance and move confirmation.\nScroll to explore this read-only page.", &[]),

    ],
    outro: "",
};

const CORRESPONDENCE: Lesson = Lesson {
    id: "online.correspondence",
    title: "Correspondence games",
    intro: &[
        "Correspondence games run over days rather than one sitting.\nPlay when it suits you; the site shows when it is your turn.",
        "The envelope icon marks correspondence games.",
    ],
    brief: "The actual custom-game form shows correspondence controls, with an envelope icon and the two clock flavours explained as a bullet list.",
    setup: Setup::Guide(GuidePage::Correspondence),
    steps: &[Step::read(
        "Two clock options:\n\n- **Days per move:** refills after each move.\n- **Total time each:** lasts the whole game.",
        &[],
    )],
    outro: "",
};

const TOURNAMENTS: Lesson = Lesson {
    id: "online.tournaments",
    title: "Tournaments",
    intro: &["Use **Tournaments** to browse events or organise one."],
    brief: "The actual tournament list and creation page, rendered read-only. Continue is the only action required.",
    setup: Setup::Guide(GuidePage::Tournaments),
    steps: &[
        Step::read("This page lists events and their details.\nPress Continue to see the creation page.", &[]),
        Step::read("Choose the event's name, rules, format and clock here.\nPress Continue to finish this read-only tour.", &[]),
    ],
    outro: "",
};

const READY: Lesson = Lesson {
    id: "online.ready",
    title: "You're ready!",
    intro: &[],
    brief: "The actual homepage is scrolled to the feedback links in its footer.",
    setup: Setup::Guide(GuidePage::Ready),
    steps: &[Step::read(
        "You're ready: play and enjoy!\nShare feedback through the links in the homepage footer.",
        &[],
    )],
    outro: "",
};

// Kept as authored material for a future release.
#[allow(dead_code)]
const SPEEDS: Lesson = Lesson {
    id: "online.speeds",
    title: "Bullet, blitz, rapid and classic",
    intro: &[
        "Games are sorted into speeds by roughly how long they last. The increment matters as much as the base time: you get it back on every move, and a typical game runs to about 40 moves each.",
        "So the site estimates each player's time as the base time plus 40 increments. Take 5+4: forty moves at 4 seconds each add 160 seconds, which is 2 minutes 40 seconds. Add the 5 minute base and you get 7 minutes 40 seconds.",
        "Under 3 minutes is bullet, under 8 minutes is blitz, under 25 minutes is rapid, and up to five hours is classic. With 7 minutes 40 seconds, 5+4 just counts as blitz.",
    ],
    brief: "No board: text only.",
    setup: Setup::TextOnly,
    steps: &[Step::read(
        "A few more: 1+0 is 1 minute, so bullet. 3+2 is 3 minutes plus 80 seconds, 4 minutes 20 seconds: blitz. 10+5 is 10 minutes plus 200 seconds, 13 minutes 20 seconds: rapid. 30+0 is 30 minutes: classic. Each speed has its own rating, and if you're new, rapid or classic gives you time to think about everything in this tutorial.",
        &[],
    )],
    outro: "",
};

#[allow(dead_code)]
const NOTATION: Lesson = Lesson {
    id: "online.notation",
    title: "Writing moves down",
    intro: &[
        "Every piece has a short name: w or b for its colour, then a letter for its kind: Q Queen [[wQ]], A Ant [[wA1]], G Grasshopper [[wG1]], B Beetle [[wB1]], S Spider [[wS1]], M Mosquito [[wM]], L Ladybug [[wL]], P Pillbug [[wP]]",
        "Kinds you have several of also get a number, in the order they entered the game. So wA1 is White's first Ant [[wA1]]",
    ],
    brief: "From a 2000+ game (corpus game 0, after four moves each), White to move. wA1 moves to -bP (highlight and arrow); Black's scripted reply is bG1 bL-. Both show in the move list, in normal turn order.",
    setup: Setup::Position("AL+Q1+M2-l-qp+a,w"),
    steps: &[
        Step::play(
            "Move your Ant [[wA1]] to the highlighted hex and watch the move list.",
            Move {
                piece: "wA1",
                to: "-bP",
            },
            "Click the Ant the arrow starts from, then the highlighted hex.",
        )
        .highlighted(&["-bP"])
        .with_arrows(&[("wA1", "-bP")])
        .with_history()
        .reply("bG1", "bL-"),
        Step::read(
            "Your move is written wA1 -bP: your first Ant [[wA1]], now on the left (-) of the black Pillbug [[bP]]. Black answered bG1 bL-: its first Grasshopper [[wG1]], placed on the right of its Ladybug [[wL]]. / and \\ mark the diagonal sides the same way.",
            &[],
        )
        .with_history(),
    ],
    outro: "Every game on the site is recorded like this, and the History tab shows it during and after a game.",
};
