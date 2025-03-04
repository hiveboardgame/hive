use crate::components::molecules::banner::Banner;
use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn Faq() -> impl IntoView {
    let i18n = use_i18n();
    let header_class = "text-lg leading-6 font-medium";
    let paragraph_class = "mt-2 text-base";
    let div_class = "p-3";
    //Helpers for links
    let rel = "external";
    let target = "_blank";
    let class = "text-blue-500 hover:underline";
    view! {
        <div class="pt-20">
            <div class="px-4 mx-auto max-w-4xl sm:px-6 lg:px-8">
                <Banner title=t!(i18n, faq.title) />

                <div class="space-y-10 md:space-y-0 md:grid md:grid-cols-1 md:gap-x-6 lg:gap-x-8">
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.what_is_hivegame.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.what_is_hivegame.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_to_help.question)}</h3>
                        <p class=paragraph_class>
                            <ul class="mt-2 list-disc list-inside">
                                <li>{t!(i18n, faq.how_to_help.answers.item1, 
                                    < source_link > = 
                                    <a href="https://github.com/hiveboardgame/hive" rel=rel target=target class=class/>
                                )}</li>
                                <li>{t!(i18n, faq.how_to_help.answers.item2,
                                     < discord_link > = 
                                     <a href="https://discord.gg/jNTjr5vj9Z" rel=rel target=target class=class/>
                                )}</li>
                                <li>{t!(i18n, faq.how_to_help.answers.item3, 
                                    < donate_link > = <a href="/donate" rel=rel target=target class=class/>
                                )}</li>
                            </ul>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_does_it_operate.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.how_does_it_operate.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_to_get_crown.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.how_to_get_crown.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.what_is_hive.question)}</h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.what_is_hive.answer, 
                                < gen42_link > = 
                                <a href="https://www.gen42.com/" rel=rel target=target class=class/>
                            )}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            {t!(i18n, faq.how_to_play_with_friends.question)}
                        </h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.how_to_play_with_friends.answers.item1)}
                        </p>
                        <p class=paragraph_class>
                            {t!(i18n, faq.how_to_play_with_friends.answers.item2)}
                        </p>
                        <p class=paragraph_class>
                            {t!(i18n, faq.how_to_play_with_friends.answers.item3)}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_to_check_stack.question)}</h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.how_to_check_stack.answers.item1)}
                        </p>
                        <p class=paragraph_class>
                            {t!(i18n, faq.how_to_check_stack.answers.item2)}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            {t!(i18n, faq.what_are_cornfirmation_modes.question)}
                        </h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.what_are_cornfirmation_modes.description)}
                            <ol class="list-decimal list-inside">
                                <li>{t!(i18n, faq.what_are_cornfirmation_modes.answers.item1)}</li>
                                <li>{t!(i18n, faq.what_are_cornfirmation_modes.answers.item2)}</li>
                                <li>{t!(i18n, faq.what_are_cornfirmation_modes.answers.item3)}</li>
                            </ol>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.can_i_play_base_game.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.can_i_play_base_game.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.what_rating_system.question)}</h3>
                        <p class=paragraph_class>

                            {t!(i18n, faq.what_rating_system.answer, 
                                < glicko2_link > =
                                <a href="https://wikipedia.org/wiki/Glicko-2" rel=rel target=target class=class/>
                            )}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_to_recover_password.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.how_to_recover_password.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.why_basic_chat.question)}</h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.why_basic_chat.details)}
                            <ol class="list-decimal list-inside">
                                <li>{t!(i18n, faq.why_basic_chat.answers.item1)}</li>
                                <li>{t!(i18n, faq.why_basic_chat.answers.item2)}</li>
                                <li>{t!(i18n, faq.why_basic_chat.answers.item3)}</li>
                            </ol>
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.opponent_is_abusive.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.opponent_is_abusive.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            {t!(i18n, faq.where_to_meet_other_players.question)}
                        </h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.where_to_meet_other_players.answer, 
                                < resources_link >
                                = <a href="/resources" rel=rel target=target class=class/>
                            )}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            {t!(i18n, faq.why_elo_has_questionmark.question)}
                        </h3>
                        <p class=paragraph_class>{t!(i18n, faq.why_elo_has_questionmark.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.why_not_in_top_players.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.why_not_in_top_players.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.how_to_review_games.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.how_to_review_games.answer)}</p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.where_to_learn_more.question)}</h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.where_to_learn_more.answer, 
                                < resources_link > =
                                <a href="/resources" rel=rel target=target class=class/>
                            )}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>
                            {t!(i18n, faq.can_i_set_up_a_tournament.question)}
                        </h3>
                        <p class=paragraph_class>
                            {t!(i18n, faq.can_i_set_up_a_tournament.answer)}
                        </p>
                    </div>
                    <div class=div_class>
                        <h3 class=header_class>{t!(i18n, faq.can_i_play_with_bots.question)}</h3>
                        <p class=paragraph_class>{t!(i18n, faq.can_i_play_with_bots.answer)}</p>
                    </div>
                </div>
            </div>
        </div>
    }
}
