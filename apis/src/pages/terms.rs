use crate::{
    common::{GEN42_URL, HIVEGAME_DISCORD_URL, LICENSE_URL},
    components::layouts::legal_page::{ContactMaintainers, LegalList, LegalPage, LegalSection},
};
use leptos::prelude::*;

#[component]
pub fn Terms() -> impl IntoView {
    view! {
        <LegalPage
            title="Terms of Service"
            last_updated="16 October 2026"
            summary="hivegame.com is a free, non-commercial place to play Hive online, run by volunteers. By using the site you agree to these terms. If you don't agree, please don't use it."
        >
            <LegalSection title="Your account">
                <LegalList>
                    <li>
                        "Keep your password safe. You are responsible for what happens on your account."
                    </li>
                    <li>
                        "Use an email address that you control, so you can recover your account."
                    </li>
                    <li>
                        "Usernames must not be offensive or impersonate someone else. We may rename or close accounts that break this rule."
                    </li>
                </LegalList>
            </LegalSection>

            <LegalSection title="Fair play">
                <LegalList>
                    <li>
                        "Play your own games. Don't use engines, bots or other people to help you during a game."
                    </li>
                    <li>"Bots are welcome, but only on accounts that we have marked as bots."</li>
                    <li>
                        "Don't manipulate ratings, for example by losing on purpose or by playing against your own accounts."
                    </li>
                </LegalList>
            </LegalSection>

            <LegalSection title="Be respectful">
                <p>
                    "Be nice to other players, as you agreed when you signed up. No harassment, hate speech, threats, spam, or sharing other people's private information. Please report abuse on our "
                    <a
                        class="ui-text-link"
                        href=HIVEGAME_DISCORD_URL
                        rel="external"
                        target="_blank"
                    >
                        "Discord server"
                    </a> "."
                </p>
            </LegalSection>

            <LegalSection title="Moderation">
                <p>
                    "We may remove content, adjust ratings, end games, and suspend or close accounts that break these terms. We try to be fair and you can always talk to us, but moderation decisions are ours to make."
                </p>
            </LegalSection>

            <LegalSection title="Your content and game records">
                <p>
                    "You keep the rights to what you write. By posting a message, you allow us to store it and show it to the people in that conversation."
                </p>
                <p>
                    "Game records (moves, results and ratings) are public. We and others may display, share and publish them."
                </p>
            </LegalSection>

            <LegalSection title="Open source and trademarks">
                <p>
                    "The hivegame.com source code is licensed under the "
                    <a class="ui-text-link" href=LICENSE_URL rel="external" target="_blank">
                        "GNU Affero General Public License v3.0"
                    </a> "."
                </p>
                <p>
                    "Hive® is a registered trademark of "
                    <a class="ui-text-link" href=GEN42_URL rel="external" target="_blank">
                        "Gen42 Games"
                    </a>
                    ". The official Hive artwork belongs to Gen42 Games and is not covered by our open-source license."
                </p>
            </LegalSection>

            <LegalSection title="Donations">
                <p>
                    "Donations are voluntary and handled by Ko-fi or Patreon under their own terms. They don't buy features, priority or influence. Donors get a 👑 as a thank-you, and nothing else changes."
                </p>
            </LegalSection>

            <LegalSection title="No warranty">
                <p>
                    "We provide hivegame.com free of charge and as it is. We do our best to keep it running, but we can't promise that it will always be available or free of errors, or that games and data will never be lost. We may change or discontinue features at any time."
                </p>
                <p>
                    "To the extent permitted by law, we are not liable for damages arising from your use of the site."
                </p>
            </LegalSection>

            <LegalSection title="Changes and contact">
                <p>
                    "We may update these terms. The date at the top of this page shows the current version. If you keep using the site after a change, the updated terms apply."
                </p>
                <p>"If you have questions, " <ContactMaintainers /> "."</p>
            </LegalSection>
        </LegalPage>
    }
}
