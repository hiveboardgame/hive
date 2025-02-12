use crate::components::{atoms::simple_link::SimpleLink, molecules::banner::Banner};
use leptos_i18n::*;
use leptos::prelude::*;

#[component]
pub fn Resources() -> impl IntoView {
    let i18n = use_i18n();
    let header_class = "text-2xl font-semibold mb-4";
    let list_class = "space-y-2";
    let tournaments_site =
        |children| view! { <SimpleLink link="https://www.worldhivetournaments.com/" children /> };
    let entomology_link =
        |children| view! { <SimpleLink link="https://entomology.gitlab.io/ " children /> };
    let explorer_link = |children| view! { <SimpleLink link="https://hive.bot.nu/" children /> };
    let hexketch_link = |children| view! { <SimpleLink link="https://hextech.net/" children /> };
    let mzinga_link =
        |children| view! { <SimpleLink link="https://github.com/jonthysell/Mzinga" children /> };
    let nokamute_link =
        |children| view! { <SimpleLink link="https://github.com/edre/nokamute" children /> };
    let bga_downloader_link =
        |children| view! { <SimpleLink link="https://github.com/DavidEGx/Hive-bga2bs" children /> };
    let discord_link =
        |children| view! { <SimpleLink link="https://discord.gg/djdQZPFa7E" children /> };
    let reddit_link =
        |children| view! { <SimpleLink link="https://www.reddit.com/r/hive/" children /> };
    let facebook_link = |children| {
        view! { <SimpleLink link="https://www.facebook.com/groups/hivetheboardlessgame" children /> }
    };
    let instagram_link = |children: ChildrenFn| {
        view! { <SimpleLink link="https://www.instagram.com/world_hive_community/" children=children /> }
    };
    let hive_champion_link = |children| {
        view! { <SimpleLink link="https://sites.google.com/site/playhivelikeachampion/home" children /> }
    };
    let hive_canon_link = |children| {
        view! {
            <SimpleLink
                link="https://www.lulu.com/de/shop/joe-schultz/the-canon-of-hive/ebook/product-1pgjmv8d.html"
                children
            />
        }
    };
    let hive_puzzles_link = |children| {
        view! { <SimpleLink link="https://gripot.se/hive/HivePuzzles_vol1.pdf" children /> }
    };
    let frasco_link = |children| {
        view! { <SimpleLink link="https://www.youtube.com/@AdAbstraGames" children /> }
    };
    let ringersol_link = |children| {
        view! { <SimpleLink link="https://www.youtube.com/playhivelikeachampion" children /> }
    };
    let ordep_cubik_link = |children| {
        view! { <SimpleLink link="https://www.youtube.com/@OrdepCubik" children /> }
    };
    let cavaliers16_link = |children| {
        view! { <SimpleLink link="https://www.twitch.tv/cavaliers16" children /> }
    };
    let gen42_link = |children| {
        view! { <SimpleLink link="https://www.gen42.com/" children /> }
    };
    let facebook_gen42_link = |children| {
        view! { <SimpleLink link="https://www.facebook.com/HiveGen42Games" children /> }
    };
    let instagram_gen42_link = |children| {
        view! { <SimpleLink link="https://www.instagram.com/gen42games/" children=children /> }
    };
    let youtube_gen42_link = |children| {
        view! { <SimpleLink link="https://www.youtube.com/@Gen42games" children=children /> }
    };
    //TODO: uncomment out the stuff
    view! {
        <div class="px-4 pt-20">
            <div class="container px-4 mx-auto">
                <Banner title={ t!(i18n, resources.title).into_any() } />
                <div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3">
                    <section>
                        <h2 class=header_class>{t!(i18n, resources.hive_news.title)}</h2>
                        <ul class=list_class>
                            <li>
                                //{t!(i18n, resources.hive_news.description, < tournaments_site >)}
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.online_tools.title)}</h2>
                        <ul class=list_class>
                            <li>
                                //{t!(i18n, resources.online_tools.tools.item1, < entomology_link >)}
                            </li>
                            <li>
                                //{t!(i18n, resources.online_tools.tools.item2, < explorer_link >)}
                            </li>
                            <li>
                                //{t!(i18n, resources.online_tools.tools.item3, < hexketch_link >)}
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.offline_tools.title)}</h2>
                        <ul class=list_class>
                            <li>
                                //{t!(i18n, resources.offline_tools.tools.item1, < mzinga_link >)}
                            </li>
                            <li>
                                //{t!(i18n, resources.offline_tools.tools.item2, < nokamute_link >)}
                            </li>
                            <li>
                                //{t!(
                                //    i18n, resources.offline_tools.tools.item3, < bga_downloader_link
                                //    >
                                //)}

                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.social_media.title)}</h2>
                        <ul class=list_class>
                            <li>
                                //{t!(i18n, resources.social_media.links.item1, < discord_link >)}
                            </li>
                            //<li>{t!(i18n, resources.social_media.links.item2, < reddit_link >)}</li>
                            <li>
                                //{t!(i18n, resources.social_media.links.item3, < facebook_link >)}
                            </li>
                            <li>
                                //{t!(i18n, resources.social_media.links.item4, < instagram_link >)}
                            </li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.books.title)}</h2>
                        <ul class=list_class>
                            //<li>{t!(i18n, resources.books.links.item1, < hive_champion_link >)}</li>
                            //<li>{t!(i18n, resources.books.links.item2, < hive_canon_link >)}</li>
                            //<li>{t!(i18n, resources.books.links.item3, < hive_puzzles_link >)}</li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.videos.title)}</h2>
                        <ul class=list_class>
                            //<li>{t!(i18n, resources.videos.links.item1, < frasco_link >)}</li>
                            //<li>{t!(i18n, resources.videos.links.item2, < ringersol_link >)}</li>
                            //<li>{t!(i18n, resources.videos.links.item3, < ordep_cubik_link >)}</li>
                            //<li>{t!(i18n, resources.videos.links.item4, < cavaliers16_link >)}</li>
                        </ul>
                    </section>

                    <section>
                        <h2 class=header_class>{t!(i18n, resources.publisher.title)}</h2>
                        <ul class=list_class>
                            //<li>{t!(i18n, resources.publisher.links.item1, < gen42_link >)}</li>
                            <li>
                                //{t!(i18n, resources.publisher.links.item2, < facebook_gen42_link >)}
                            </li>
                            //<li>{t!(i18n, resources.publisher.links.item3, < facebook_link >)}</li>
                            <li>
                                //{t!(
                                //    i18n, resources.publisher.links.item4, < instagram_gen42_link >
                                //)}

                            </li>
                            <li>
                                //{t!(i18n, resources.publisher.links.item5, < youtube_gen42_link >)}
                            </li>
                        </ul>
                    </section>
                </div>
            </div>
        </div>
    }
}
