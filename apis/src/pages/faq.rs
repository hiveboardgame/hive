use leptos::*;

use crate::components::molecules::banner::Banner;

#[component]
pub fn Faq() -> impl IntoView {
    let header_class = "text-lg leading-6 font-medium";
    let paragraph_class = "mt-2 text-base";
    let div_class = "p-3";
    view! {
        <div class="pt-20">
            <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
                <Banner title="Frequently asked questions"/>
                <div class="space-y-10 md:space-y-0 md:grid md:grid-cols-1 md:gap-x-6 lg:gap-x-8">
                    <div class=div_class>
                        <h3 class=header_class>"What is hivegame.com?"</h3>
                        <p class=paragraph_class>
                            "hivegame.com is a community effort to create a new online platform to play Hive® for free. hivegame.com is ad-free, open source, developed by members of the community, and officially supported by the creator of Hive® John Yianni."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"How can I help this project thrive?"</h3>
                        <p class=paragraph_class>
                            <ul class="list-disc list-inside">
                                <li>
                                    You can help us by contributing to the
                                    <a
                                        href="https://github.com/hiveboardgame/hive"
                                        rel="external"
                                        target="_blank"
                                        class="text-blue-500 hover:underline"
                                    >
                                        code base.
                                    </a>
                                </li>
                                <li>
                                    You can support us with ideas, feedback, and graphics check on
                                    <a
                                        href="https://discord.gg/jNTjr5vj9Z"
                                        rel="external"
                                        target="_blank"
                                        class="text-blue-500 hover:underline"
                                    >
                                        Discord
                                    </a>
                                </li>
                                <li>
                                    You can keep the lights on with a
                                    <a href="/donate" class="text-blue-500 hover:underline">
                                        donation.
                                    </a>
                                    "And once the lights are always on, your donation helps keeping the developers' lights on."
                                </li>
                            </ul>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"How does hivegame.com operate?"</h3>
                        <p class=paragraph_class>
                            "It's developed by communtity members, and 100% donation based. hivegame.com will never collect any user data, show ads or implement anything else that will distract you from the game or makes money."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"How do I get a 👑 next to my username?"</h3>
                        <p class=paragraph_class>"By donating to the project!"</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"What is Hive?"</h3>
                        <p class=paragraph_class>
                            "Hive is an award-winning abstract strategy game designed by John Yianni in 2000 and published by"
                            <a
                                href="https://www.gen42.com/"
                                rel="external"
                                target="_blank"
                                class="text-blue-500 hover:underline"
                            >
                                Gen42.
                            </a>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Can I set up a game with a friend?"</h3>
                        <p class=paragraph_class>
                            "Yes, you can create a game by clicking on the 'create a game' button of the main page, then select the ‘private’ option and afterwards share the link with a friend. They will need to have/create account to play."
                        </p>
                        <p class=paragraph_class>
                            "Alternatively, you can send a direct challenge by using the 'crossed swords' icon next to each user, that will send a direct challenge to that specific user."
                        </p>
                        <p class=paragraph_class>
                            "Of course, you can also set up an open challenge that can be accepted by anyone in the Elo range criteria and just hope they are first to accept your game."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"How do I see what is under a piece"</h3>
                        <p class=paragraph_class>
                            "On mobile, longpress on the stack to expand it. After it is expanded, you can move your finger out of the way while staying in contact with the screen to see the lowest piece as well."
                        </p>
                        <p class=paragraph_class>
                            "On desktop, right click on the stack and it will stay expanded until you let go."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"What do the different confirmation modes mean?"</h3>
                        <p class=paragraph_class>
                            "When making a move there are 3 modes:"
                            <ol class="list-decimal list-inside">
                                <li>
                                    "Double click - The default one, the first click places the piece and the second click makes the move."
                                </li>
                                <li>
                                    "Single click - The fast one, if you want to not lose time in fast games. Careful not to misplace because the one and only click immediately makes the move."
                                </li>
                                <li>
                                    "Clock confirm - The secure one, if you want to avoind making moves by mistake. First click where to place/move and then to confirm and make the move you have to click on your own timer."

                                </li>
                            </ol>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Can I play the base game without expansions?"</h3>
                        <p class=paragraph_class>
                            "Yes, but why would you? Bad joke aside, the base game is restricted to unrated play. Hive PLM (all three expansions: Pillbug, Ladybug, Mosquito) is the offical format used in tournaments. It's also the most challenging and fun!"
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"What rating system does hivegame.com use?"</h3>
                        <p class=paragraph_class>
                            "We use the "
                            <a
                                href="https://en.wikipedia.org/wiki/Glicko_rating_system"
                                rel="external"
                                target="_blank"
                                class="text-blue-500 hover:underline"
                            >
                                Glicko-2
                            </a> " system. Parameterized like Lichess."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"What can I do if I forgot my password?"</h3>
                        <p class=paragraph_class>
                            "We don't have automatic password recovery yet, you can contact us via Discord @klautcomputing / @ionoi and ask us to reset your password."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Why is chat so basic?"</h3>
                        <p class=paragraph_class>
                            "The chat feature is very much a work in progress, we are aware it has issues, important things to keep in mind when using it:"
                            <ol class="list-decimal list-inside">
                                <li>
                                    "Currently the chat is not persisted in the database so whenever we deploy an update your chat messages will be lost!"
                                </li>
                                <li>
                                    "For now it is only possible to chat in a game (direct messages and other ways will be added), spectators don't see the player chat nor do the players see the spectator chat."
                                </li>
                                <li>
                                    "Sometimes the red chat notification will show new messages when there aren't any."
                                </li>
                            </ol>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            "My opponent is abusive or disruptive in game/chat or has an innapropriate username, what can I do?"
                        </h3>
                        <p class=paragraph_class>
                            "Screenshot the behaviour and contact us via Discord @klautcomputing / @ionoi we will sort it out."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            "Where can I get to know the rest of the community playing Hive® all around the world?"
                        </h3>
                        <p class=paragraph_class>
                            "Find the links to our social media "
                            <a
                                href="/resources"
                                target="_blank"
                                class="text-blue-500 hover:underline"
                            >
                                here
                            </a>
                            "The Hive® community is very friendly and you will surely find someone close to you to play face to face as well!"
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Why does my elo have a “?” behind it?"</h3>
                        <p class=paragraph_class>
                            "This happens while your elo is still uncertain. It goes away once you have played enough games and once the ? is gone you can also show up on the leaderboards."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            "Why do I not show up in the top ranked players?"
                        </h3>
                        <p class=paragraph_class>
                            "You haven’t played enough games yet for the system to be really sure how accurate your rating is. Just keep playing a couple games more, hopefully win them and you will show up."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            "Can I review finished games with an analysis tool?"
                        </h3>
                        <p class=paragraph_class>
                            "Yes, finished games have a microscope icon that will load the game into analysis mode but this feature is currently under development, so don’t expect too much yet."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            "How can I learn more about the strategy of Hive?"
                        </h3>
                        <p class=paragraph_class>
                            "We are planning on adding an interactive tutorial to help you learn and understand all the strategic ideas behind the game, in the meantime check the "
                            <a
                                href="/resources"
                                target="_blank"
                                class="text-blue-500 hover:underline"
                            >
                                resources
                            </a> "section for books, videos and the discord!"
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Can I set up a tournament?"</h3>
                        <p class=paragraph_class>
                            "Not yet, unless you run the tournament yourself."
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>"Can I play against a bot?"</h3>
                        <p class=paragraph_class>
                            "This feature is not supported yet, but it could be soon (get in contact with us if you want to help code this)."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}
