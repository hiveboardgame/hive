use crate::{
    common::{GEN42_URL, HIVEGAME_DISCORD_URL, LICENSE_URL},
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
};
use leptos::prelude::*;

const LAST_UPDATED: &str = "16 October 2026";
const SECTION_CLASS: &str = "space-y-3 text-sm leading-6 text-gray-700 dark:text-gray-300";
const LIST_CLASS: &str = "space-y-2 pl-5 list-disc";

#[component]
pub fn Terms() -> impl IntoView {
    view! {
        <PageShell>
            <PageHeader title="Terms of Service" subtitle=format!("Last updated {LAST_UPDATED}") />
            <p class="ui-notice">
                "hivegame.com is a free, non-commercial place to play Hive online, run by volunteers. By using the site you agree to these terms. If you don't agree, please don't use it."
            </p>

            <Panel title="Your account" body_class=SECTION_CLASS>
                <ul class=LIST_CLASS>
                    <li>
                        "Keep your password safe. You are responsible for what happens on your account."
                    </li>
                    <li>
                        "Use an email address that you control, so you can recover your account."
                    </li>
                    <li>
                        "Usernames must not be offensive or impersonate someone else. We may rename or close accounts that break this rule."
                    </li>
                </ul>
            </Panel>

            <Panel title="Fair play" body_class=SECTION_CLASS>
                <ul class=LIST_CLASS>
                    <li>
                        "Play your own games. Don't use engines, bots or other people to help you during a game."
                    </li>
                    <li>"Bots are welcome, but only on accounts that we have marked as bots."</li>
                    <li>
                        "Don't manipulate ratings, for example by losing on purpose or by playing against your own accounts."
                    </li>
                </ul>
            </Panel>

            <Panel title="Be respectful" body_class=SECTION_CLASS>
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
            </Panel>

            <Panel title="Moderation" body_class=SECTION_CLASS>
                <p>
                    "We may remove content, adjust ratings, end games, and suspend or close accounts that break these terms. We try to be fair and you can always talk to us, but moderation decisions are ours to make."
                </p>
            </Panel>

            <Panel title="Your content and game records" body_class=SECTION_CLASS>
                <p>
                    "You keep the rights to what you write. By posting a message, you allow us to store it and show it to the people in that conversation."
                </p>
                <p>
                    "Game records (moves, results and ratings) are public. We and others may display, share and publish them."
                </p>
            </Panel>

            <Panel title="Open source and trademarks" body_class=SECTION_CLASS>
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
            </Panel>

            <Panel title="Donations" body_class=SECTION_CLASS>
                <p>
                    "Donations are voluntary and handled by Ko-fi or Patreon under their own terms. They don't buy features, priority or influence. Donors get a 👑 as a thank-you, and nothing else changes."
                </p>
            </Panel>

            <Panel title="No warranty" body_class=SECTION_CLASS>
                <p>
                    "We provide hivegame.com free of charge and as it is. We do our best to keep it running, but we can't promise that it will always be available or free of errors, or that games and data will never be lost. We may change or discontinue features at any time."
                </p>
                <p>
                    "To the extent permitted by law, we are not liable for damages arising from your use of the site."
                </p>
            </Panel>

            <Panel title="Changes and contact" body_class=SECTION_CLASS>
                <p>
                    "We may update these terms. The date at the top of this page shows the current version. If you keep using the site after a change, the updated terms apply."
                </p>
                <p>
                    "Questions? Message leex or iongler on our "
                    <a
                        class="ui-text-link"
                        href=HIVEGAME_DISCORD_URL
                        rel="external"
                        target="_blank"
                    >
                        "Discord server"
                    </a> "."
                </p>
            </Panel>
        </PageShell>
    }
}
