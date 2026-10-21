use super::{
    goal::Goal::*,
    lesson::{Chapter, GuidePage, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "online",
    title: "Playing online",
    summary: "Find a game, use the clock and controls, and make yourself at home.",
    lessons: &[CREATE_JOIN, TIME_CONTROLS, CONTROLS, SETTINGS, CORRESPONDENCE, TOURNAMENTS, READY],
};

const CREATE_JOIN: Lesson = Lesson {
    id: "online.create_join",
    title: "A bit more about hivegame.com",
    intro: &["The homepage gives you three ways to start playing: choose a ready-made time control, create a custom game, or join an open game."],
    brief: "Actual homepage, rendered read-only. Point to quick games, Custom, and open challenges in three read-only stages.",
    setup: Setup::Guide(GuidePage::Home),
    steps: &[
        Step::read("Choose one of these ready-made time controls to start a game quickly. The two numbers show minutes per player and seconds added after each move.", &[]),
        Step::read("Choose Custom to set your own time control and game options, including whether the game is rated and which colour you prefer.", &[]),
        Step::read("You can also join a game someone else has created. Check the time control and game details in the open challenges list, then choose Join.", &[]),
    ],
    outro: "Pick a comfortable time control for your first games, so you have time to think.",
};

const TIME_CONTROLS: Lesson = Lesson {
    id: "online.time_controls",
    title: "Reading a time control",
    intro: &[
        "A time control has two numbers. In 5+4, each player starts with 5 minutes and gains 4 seconds after every move.",
        "Here we use 1+0: one minute each, with no increment. Your clock runs only during your turn. Reaching zero loses the game.",
        "Make a move before your minute runs out, then watch Black's clock count down.",
    ],
    brief: "A local 1+0 demonstration using the existing four-ply board. Any legal White turn starts Black's clock. Black makes no move and loses at zero; a White timeout requires retry.",
    setup: Setup::Clocked {
        moves: "wL;bL wL-;wM -wL;bM bL-",
        base_minutes: 1,
        increment_seconds: 0,
    },
    steps: &[
        Step::play(
            "You play as White. Both players have one minute. Make any legal move or placement before your clock reaches zero, then watch your opponent's clock.",
            PlayAny,
            "Make a legal White move before your minute runs out.",
        ).await_opponent(),
        Step::play(
            "Your clock has stopped and Black's clock is running. Black will not move in this exercise: watch what happens when the clock reaches zero.",
            WaitForTimeout(Color::Black),
            "Wait for Black's clock to reach zero.",
        ).await_opponent(),
        Step::read("Black ran out of time, so White wins! The red clock shows which player lost on time.", &[]).announce_winner(),
    ],
    outro: "",
};

const CONTROLS: Lesson = Lesson {
    id: "online.controls",
    title: "Undo, offer a draw and resign",
    intro: &["These three controls appear beside the board. In a real game, the first click selects an action and a second click confirms it."],
    brief: "The actual board, reserves and game-control components, with each control highlighted in its normal sidebar position.",
    setup: Setup::Position("L+l+Q+q,w"),
    steps: &[
        Step::read("Undo requests a takeback of your last move. Your opponent can accept or decline the request; a request does not undo a move automatically.", &[]),
        Step::read("Offer draw asks your opponent to agree to a draw. If they accept, the game ends with no winner. If they decline, the game continues.", &[]),
        Step::read("Resign ends the game immediately and gives your opponent the win. Use it when you decide you no longer want to continue the game.", &[]),
    ],
    outro: "",
};

const SETTINGS: Lesson = Lesson {
    id: "online.settings",
    title: "Change settings",
    intro: &["Open your profile menu at the top right and choose Config to find your settings."],
    brief: "The actual Settings component is rendered read-only in a script-free frame.",
    setup: Setup::Guide(GuidePage::Settings),
    steps: &[
        Step::read("This is the actual Settings page. You can reach it by opening your profile menu at the top right and choosing Config.", &[]),

    ],
    outro: "You can return to Config from your profile menu whenever you want.",
};

const CORRESPONDENCE: Lesson = Lesson {
    id: "online.correspondence",
    title: "Correspondence games",
    intro: &[
        "Real-time games are played in one sitting. Correspondence games are played over days: make your move when it suits you, and the site tells you when it is your turn.",
        "Look for the envelope symbol to recognise correspondence games.",
    ],
    brief: "The actual custom-game form shows correspondence controls, with an envelope icon and the two clock flavours explained as a bullet list.",
    setup: Setup::Guide(GuidePage::Correspondence),
    steps: &[Step::read(
        "Correspondence comes in two flavours:
- **Days per move:** you have that many days for each move. The allowance refills after every move.
- **Total days:** each player has that many days for the whole game.
You can also play untimed games with no clock.",
        &[],
    )],
    outro: "",
};

const TOURNAMENTS: Lesson = Lesson {
    id: "online.tournaments",
    title: "Tournaments",
    intro: &["The Tournaments menu lets you browse events and organise your own."],
    brief: "The actual tournament list and creation page, rendered read-only. Continue is the only action required.",
    setup: Setup::Guide(GuidePage::Tournaments),
    steps: &[
        Step::read("Browse tournaments here to see their status, time control and participants. Open an event in the real site to read its rules before joining.", &[]),
        Step::read("Create Tournament lets an organiser choose a name, description, format, player limits, time control and start rules. When you organise an event, explain its rules clearly to everyone joining.", &[]),
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
        "You've learned everything you need to get started. Now comes the best part: play and enjoy! Let us know if you have any feedback using the links in the homepage footer.",
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
        "Every piece has a short name: w or b for its colour, then a letter for its kind: Q Queen [[wQ]], A Ant [[wA1]], G Grasshopper [[wG1]], B Beetle [[wB1]], S Spider [[wS1]], M Mosquito [[wM]], L Ladybug [[wL]], P Pillbug [[wP]].",
        "Kinds you have several of also get a number, in the order they entered the game. So wA1 is White's first Ant [[wA1]].",
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
