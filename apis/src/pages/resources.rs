use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
    i18n::*,
};
use leptos::prelude::*;

#[component]
pub fn Resources() -> impl IntoView {
    let i18n = use_i18n();
    let list_class = "space-y-2 text-sm leading-6 text-gray-700 dark:text-gray-300";
    //Helpers for links
    let rel = "external";
    let target = "_blank";
    let class = "ui-text-link";

    view! {
        <PageShell>
            <PageHeader title=move || { t_string!(i18n, resources.title) } />
            <div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3">
                <Panel title=move || { t_string!(i18n, resources.hive_news.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.hive_news.description,
                                    < tournaments_site > =
                                    <a href="https://www.worldhivetournaments.com/" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.online_tools.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.online_tools.tools.item1,
                                    < entomology_link > =
                                    <a href="https://entomology.gitlab.io/" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.online_tools.tools.item2,
                                    < explorer_link > =
                                    <a href="https://hive.bot.nu/" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.online_tools.tools.item3,
                                    < hexketch_link > =
                                    <a href="https://github.com/qmolt/hexketch" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.offline_tools.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.offline_tools.tools.item1,
                                    < mzinga_link > =
                                    <a href="https://github.com/jonthysell/Mzinga" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.offline_tools.tools.item2,
                                    < nokamute_link > =
                                    <a href="https://github.com/edre/nokamute" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.offline_tools.tools.item3,
                                    < bga_downloader_link > =
                                    <a href="https://github.com/DavidEGx/Hive-bga2bs" rel=rel target=target class=class/>
                            )}

                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.social_media.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.social_media.links.item1,
                                    < discord_link > =
                                    <a href="https://discord.gg/djdQZPFa7E" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.social_media.links.item2,
                                < reddit_link > =
                                <a href="https://www.reddit.com/r/hive/" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.social_media.links.item3,
                                    < facebook_link > =
                                    <a href="https://www.facebook.com/groups/hivetheboardlessgame" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.social_media.links.item4,
                                    < instagram_link >
                                    = <a href="https://www.instagram.com/world_hive_community/" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.books.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.books.links.item1,
                                < hive_champion_link > =
                                <a href="https://sites.google.com/site/playhivelikeachampion/home" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.books.links.item2,
                                < hive_canon_link > =
                                <a href="https://www.lulu.com/de/shop/joe-schultz/the-canon-of-hive/ebook/product-1pgjmv8d.html" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.books.links.item3,
                                < hive_puzzles_link > =
                                <a href="https://gripot.se/hive/HivePuzzles_vol1.pdf" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.videos.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.videos.links.item1,
                                < frasco_link > =
                                <a href="https://www.youtube.com/@FrascoAdAbstra" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.videos.links.item2,
                                < ringersol_link > =
                                <a href="https://www.youtube.com/playhivelikeachampion" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.videos.links.item3,
                                < ordep_cubik_link > =
                                <a href="https://www.youtube.com/@OrdepCubik" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.videos.links.item4,
                                < cavaliers16_link > =
                                <a href="https://www.twitch.tv/cavaliers16" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>

                <Panel title=move || { t_string!(i18n, resources.publisher.title) }>
                    <ul class=list_class>
                        <li>
                            {t!(
                                i18n, resources.publisher.links.item1,
                                < gen42_link > =
                                <a href="https://www.gen42.com/" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.publisher.links.item2,
                                    < facebook_gen42_link > =
                                    <a href="https://www.facebook.com/HiveGen42Games" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.publisher.links.item3,
                                < facebook_link > =
                                <a href="https://www.facebook.com/groups/hivetheboardlessgame" rel=rel target=target class=class/>
                            )}
                        </li>
                        <li>
                            {t!(
                                i18n, resources.publisher.links.item4,
                                    < instagram_gen42_link > =
                                    <a href="https://www.instagram.com/gen42games/" rel=rel target=target class=class/>
                            )}

                        </li>
                        <li>
                            {t!(
                                i18n, resources.publisher.links.item5,
                                    < youtube_gen42_link > =
                                    <a href="https://www.youtube.com/@Gen42games" rel=rel target=target class=class/>
                            )}
                        </li>
                    </ul>
                </Panel>
            </div>
        </PageShell>
    }
}
