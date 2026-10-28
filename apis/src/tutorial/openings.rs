use super::{
    goal::{Condition::*, Goal::*},
    lesson::{Chapter, Lesson, Setup, Step},
};

pub const CHAPTER: Chapter = Chapter {
    id: "openings",
    title: "Standard openings",
    summary: "How strong players start a game, and why.",
    lessons: &[LADYBUG, DEVELOPMENT, PILLBUG],
};

const LADYBUG: Lesson = Lesson {
    id: "openings.ladybug",
    title: "The Ladybug opening",
    intro: &[
        "In games between strong players on this site, White opens with the Ladybug about seven times in ten, and Black usually answers with their own Ladybug.",
        "The Ladybug makes a good first piece: it's hard to trap and doesn't need room around it, and your Ants stay in reserve where they can't be pinned yet.",
    ],
    brief: "Ladybug opening after one move each (wL, bL wL-). White to move; the most common choice at 2000+ is the Mosquito.",
    setup: Setup::Position("L+l,w"),
    steps: &[Step::play(
        "The most popular second piece here is the Mosquito. Place it next to your Ladybug.",
        Reach(Placed("wM")),
        "Choose your Mosquito from the reserve, then a highlighted hex.",
    )],
    outro: "Next to the Ladybug, the Mosquito can already copy its moves, and later it can borrow from whatever else you bring in.",
};

const DEVELOPMENT: Lesson = Lesson {
    id: "openings.development",
    title: "Bringing in an Ant",
    intro: &[
        "After Ladybugs and Mosquitoes on both sides, strong players most often bring in an Ant next, with the Queen close behind.",
        "An Ant placed now has the whole outside of the hive to run around on before things get crowded.",
    ],
    brief: "Ladybug opening after two moves each (wL, bL wL-, wM -wL, bM bL-). White to move; the most common choice at 2000+ is an Ant.",
    setup: Setup::Position("M+Llm,w"),
    steps: &[Step::play(
        "Bring an Ant into play.",
        Reach(Placed("wA")),
        "Choose an Ant from your reserve, then a highlighted hex.",
    )],
    outro: "Your Queen still has to arrive by your fourth turn, and most players place her on their third or fourth.",
};

const PILLBUG: Lesson = Lesson {
    id: "openings.pillbug",
    title: "The Pillbug opening",
    intro: &[
        "The second most popular start is the Pillbug. After Black answers with the Ladybug, White almost always places the Queen straight away, right next to the Pillbug.",
        "That puts the Pillbug where it is most useful: beside its own Queen, ready to lift her out of trouble.",
    ],
    brief: "Pillbug opening after one move each (wP, bL wP-). White to move; at 2000+ the Queen comes next in almost every game.",
    setup: Setup::Position("P+l,w"),
    steps: &[Step::play(
        "Place your Queen next to your Pillbug.",
        Reach(All(&[Placed("wQ"), Touching("wQ", "wP")])),
        "Choose your Queen from the reserve and place her touching the Pillbug.",
    )],
    outro: "With the Pillbug next to the Queen, any surround has to deal with it first. There are many more openings to explore, and Frasco's YouTube channel is a great place to learn them: https://www.youtube.com/@FrascoAdAbstra",
};
