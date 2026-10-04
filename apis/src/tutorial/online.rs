use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};
use hive_lib::Color;

pub const CHAPTER: Chapter = Chapter {
    id: "online",
    title: "Playing online",
    summary: "Clocks, time controls and correspondence games on hivegame.com.",
    lessons: &[NOTATION, TIME_CONTROLS, SPEEDS, CORRESPONDENCE],
};

const TIME_CONTROLS: Lesson = Lesson {
    id: "online.time_controls",
    title: "Reading a time control",
    intro: &[
        "Most games here are played with a clock. A time control like 5+4 has two numbers.",
        "The first is your base time in minutes: you start with 5 minutes on your clock. The second is the increment in seconds: every time you make a move, 4 seconds are added to your clock.",
        "Your clock only runs while it's your turn. If it reaches zero, you lose the game, however well you were doing on the board.",
    ],
    brief: "The first four moves of a 2000+ game (corpus game 19), loaded as moves so the History tab has a real history. White to move, on a running 5+4 clock. The right column is the play page's side board. Any white placement works; Black's scripted reply bA1 bM/ is legal after every one of them.",
    setup: Setup::Clocked {
        moves: "wL;bL wL-;wM -wL;bM bL-",
        base_minutes: 5,
        increment_seconds: 4,
    },
    steps: &[
        Step::read(
            "Next to the board are the clocks you'll see in every game. The top clock is your opponent's, the bottom one is yours. Yours is green and counting down, because it's your move.",
            &[],
        ),
        Step::play(
            "Place a piece and watch your clock: the moment you move, 4 seconds are added to it, and your opponent's clock takes over.",
            Reach(AnyPlaced(Color::White)),
            "Choose a piece from your reserve under the Game tab, then a highlighted hex.",
        )
        .reply("bA1", "bM/"),
        Step::read(
            "Black answered straight away and got 4 seconds too. Now find the game controls, between the reserves (above the board on a phone): request a takeback, offer a draw, or resign. Try one. Just like in a real game, the first click turns it green and the second click confirms, so you can't press one by accident.",
            &[],
        ),
    ],
    outro: "Your clock kept running while you read all this. That's real-time Hive: thinking costs time, so spend it on the moves that matter.",
};

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

const CORRESPONDENCE: Lesson = Lesson {
    id: "online.correspondence",
    title: "Real-time and correspondence games",
    intro: &[
        "Real-time games are played in one sitting, with both players online and the clocks ticking in minutes and seconds.",
        "Correspondence games are played over days. You make your move whenever it suits you, and you can have many games going at once. The site tells you when it's your turn.",
    ],
    brief: "No board: text only.",
    setup: Setup::TextOnly,
    steps: &[Step::read(
        "Correspondence comes in two flavours. With days per move, you have that many days for each move, and the clock refills after every move you make. With total days, each player has that many days for the whole game, like a very slow clock. You can also play untimed games with no clock at all.",
        &[],
    )],
    outro: "That's everything you need to start playing. Good luck, and have fun in the hive!",
};

const NOTATION: Lesson = Lesson {
    id: "online.notation",
    title: "Writing moves down",
    intro: &[
        "Every piece has a short name: w or b for its colour, then a letter for its kind: Q Queen, A Ant, G Grasshopper, B Beetle, S Spider, M Mosquito, L Ladybug, P Pillbug.",
        "Kinds you have several of also get a number, in the order they entered the game. So wA1 is White's first Ant.",
    ],
    brief: "From a 2000+ game (corpus game 0, after four moves each), White to move. wA1 moves to -bP (highlight and arrow); Black's scripted reply is bG1 bL-. Both show in the move list, in normal turn order.",
    setup: Setup::Position("AL+Q1+M2-l-qp+a,w"),
    steps: &[
        Step::play(
            "Move your Ant to the highlighted hex and watch the move list.",
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
            "Your move is written wA1 -bP: your first Ant, now on the left (-) of the black Pillbug. Black answered bG1 bL-: its first Grasshopper, placed on the right of its Ladybug. / and \\ mark the diagonal sides the same way.",
            &[],
        )
        .with_history(),
    ],
    outro: "Every game on the site is recorded like this, and the History tab shows it during and after a game.",
};
