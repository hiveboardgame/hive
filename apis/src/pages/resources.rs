use leptos::*;

use crate::components::molecules::banner::Banner;

#[component]
pub fn Resources() -> impl IntoView {
    let link_class = "text-blue-500 hover:underline";
    let header_class = "text-2xl font-semibold mb-4";
    let list_class = "space-y-2";
    view! {
        <div class="pt-10 px-4">
            <div class="container mx-auto px-4 py-8">
                <Banner title="Community links"/>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
                    <section>
                        <h2 class=header_class>Hive news</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://www.worldhivetournaments.com/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    World Hive Tournaments
                                </a>
                                <span>
                                    Upcoming in person and online tournaments, tournament results, startegy articles. The place for the latest hive news.
                                </span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Online Tools</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://entomology.gitlab.io/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Entomology
                                </a>
                                <span>
                                    Position editor and game explorer for games played on BGA or Boardspace
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://hive.bot.nu/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Opening explorer
                                </a>
                                <span>Opening search for tournament boardspace games</span>
                            </li>
                            <li>
                                <a
                                    href="https://qmolt.github.io/hexketch/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Hextech
                                </a>
                                <span>
                                    Hexketch is a website that allows you to easily generate images of hexagonal tiles on a hexagonal grid
                                </span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Offline Tools</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://github.com/jonthysell/Mzinga"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Mzinga
                                </a>
                                <span>
                                    Mzinga is a collection of open-source software to play Hive, with the primary goal of building a community of developers who create Hive-playing AIs. Works on Windows Mac and Linux
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://github.com/edre/nokamute"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Nokamute
                                </a>
                                <span>
                                    Nokamute is a open source hive AI, compatible with Mzinga.
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://github.com/DavidEGx/Hive-bga2bs"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    BGA Game Downloader
                                </a>
                                <span>Download a Hive archived BGA game in sgf format.</span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Social Media</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://discord.gg/djdQZPFa7E"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Discord
                                </a>
                                <span>Join our community chat</span>
                            </li>
                            <li>
                                <a
                                    href="https://reddit.com/r/hive/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Hive subreddit
                                </a>
                                <span>Share your strategies and experiences</span>
                            </li>
                            <li>
                                <a
                                    href="https://www.facebook.com/groups/hivetheboardlessgame"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    World Hive Community on Facebook
                                </a>
                                <span>Connect with fellow players</span>
                            </li>
                            <li>
                                <a
                                    href="https://www.instagram.com/hiveworldcommunity/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Hive Instagram
                                </a>
                                <span>Pretty hive photos and memes</span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Books</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://sites.google.com/site/playhivelikeachampion/home"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Play Hive Like a Champion
                                </a>
                                <span>
                                    Hive strategy book by Randy Ingersoll the 2011 World Champion.
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://www.lulu.com/de/shop/joe-schultz/the-canon-of-hive/ebook/product-1pgjmv8d.html"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Canon of Hive: Groundwork
                                </a>
                                <span>
                                    First book part of a planned trilogy by multiple times hive world champion Joe Schultz
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://gripot.se/hive/HivePuzzles_vol1.pdf"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Hive Puzzles (free)
                                </a>
                                <span>
                                    Free collection of hive puzzles with solutions for beginners up to advanted
                                </span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Videos</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://www.youtube.com/@AdAbstraGames"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Frasco
                                </a>
                                <span>Hive and other Abstract Strategy Games specialist</span>
                            </li>
                            <li>
                                <a
                                    href="https://www.youtube.com/playhivelikeachampion"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Play Hive like a champion
                                </a>
                                <span>
                                    Youtube channel of Randy Ingersoll, tournament game reviews and strategy discussions
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://www.youtube.com/@OrdepCubik"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    OrdepCubick
                                    "🇪🇸"
                                </a>
                                <span>
                                    Game analysis in Spanish by a young up and coming hive master
                                </span>
                            </li>
                            <li>
                                <a
                                    href="https://www.twitch.tv/cavaliers16"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Cavaliers16
                                </a>
                                <span>Game reviews from one of the strongest hive players</span>
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>Gen42 - Hive Publisher</h2>
                        <ul class=list_class>
                            <li>
                                <a
                                    href="https://www.gen42.com/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Gen42 - buy a physical game here
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://www.facebook.com/HiveGen42Games"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Gen42 Hive community - Facebook
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://www.facebook.com/Gen42Games"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Gen42 - Facebook
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://www.instagram.com/gen42games/"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Gen42 - Instagram
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://www.youtube.com/@Gen42games"
                                    rel="external"
                                    target="_blank"
                                    class=link_class
                                >
                                    Gen42 - Youtube
                                </a>
                            </li>
                        </ul>
                    </section>
                </div>
            </div>
        </div>
    }
}
