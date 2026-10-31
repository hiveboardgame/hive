use crate::components::layouts::legal_page::{
    ContactMaintainers,
    LegalList,
    LegalPage,
    LegalSection,
};
use leptos::prelude::*;

#[component]
pub fn Privacy() -> impl IntoView {
    view! {
        <LegalPage
            title="Privacy Policy"
            last_updated="30 October 2026"
            summary="hivegame.com is a free, open-source Hive server run by volunteers. We collect as little as we can, we never sell your data, and we don't use ads, analytics or tracking."
        >
            <LegalSection title="Who is responsible">
                <p>
                    "hivegame.com is run by its volunteer maintainers, leex and iongler. They are responsible for your personal data under the EU General Data Protection Regulation (GDPR)."
                </p>
                <p>"To contact us, " <ContactMaintainers /> "."</p>
            </LegalSection>

            <LegalSection title="What we collect and why">
                <LegalList>
                    <li>
                        <strong>"Your account. "</strong>
                        "When you sign up we store your username, email address and password. Passwords are stored only as an Argon2 hash, never in plain text. We also store your settings, such as language, takeback and notification preferences, and the players you have blocked."
                    </li>
                    <li>
                        <strong>"Profile details (optional). "</strong>
                        "If you choose to add a flag or About text, we store it to display on your public profile. Your flag is self-selected and does not have to indicate where you live. You can change or clear these details at any time. We do not ask for a city, birth year, age or gender in profile settings."
                    </li>
                    <li>
                        <strong>"Games and ratings. "</strong>
                        "We store the games you play (moves, clocks and results), your ratings and your tournament participation, so we can run games, calculate ratings and show your history."
                    </li>
                    <li>
                        <strong>"Chat and messages. "</strong>
                        "We store the messages you send in game, tournament and global chat and in direct messages, so they can be shown to the people in the conversation. When abuse is reported, moderators may read the messages involved."
                    </li>
                    <li>
                        <strong>"Account emails. "</strong>
                        "We use your email address only for account emails, such as password resets. We never send newsletters or advertising. When an email is requested, we keep the email address and your IP address for 24 hours to prevent abuse."
                    </li>
                    <li>
                        <strong>"Connection data. "</strong>
                        "To deliver the site and run live games, our servers process your IP address, browser information and connection timing, for example to measure latency so a slow connection doesn't cost you clock time. We don't keep request logs. Our application logs warnings, errors and the progress of background jobs. These logs can include internal account or game IDs, usernames and, when an email can't be delivered, the recipient's email address, but no IP addresses."
                    </li>
                    <li>
                        <strong>"Browser notifications (optional). "</strong>
                        "If you turn on notifications, we store a push subscription for your browser (an address at your browser's push service plus encryption keys), together with the platform, app version and language. Notifications are delivered through your browser vendor's push service, for example Google, Mozilla or Apple. Their content is encrypted, so the push service cannot read it."
                    </li>
                    <li>
                        <strong>"Discord (optional). "</strong>
                        "If you link your Discord account to receive notifications there, our Discord bot stores your Discord user ID and username together with your hivegame account."
                    </li>
                    <li>
                        <strong>"Donations. "</strong>
                        "Donations are handled by Ko-fi and Patreon under their own privacy policies; we never see your payment details. To give you the 👑, we link your donation to your hivegame account using the details you or the platform share with us."
                    </li>
                </LegalList>
            </LegalSection>

            <LegalSection title="Legal bases">
                <LegalList>
                    <li>
                        <strong>"Providing the service you signed up for"</strong>
                        " (Art. 6(1)(b) GDPR): your account, games, ratings, chat and account emails."
                    </li>
                    <li>
                        <strong>"Our legitimate interest"</strong>
                        " in running a secure, fair and abuse-free service (Art. 6(1)(f) GDPR): connection data, application logs, abuse-prevention records, moderation and keeping game records public."
                    </li>
                    <li>
                        <strong>"Your consent"</strong>
                        " (Art. 6(1)(a) GDPR): optional public profile details, browser notifications and Discord linking. You can withdraw it at any time by clearing your profile details, turning notifications off or asking us to unlink Discord."
                    </li>
                </LegalList>
            </LegalSection>

            <LegalSection title="What other people can see">
                <p>
                    "Your username, profile, ratings, games, tournament results and whether you are online are public. Chat messages are visible to the other people in that chat."
                </p>
                <p>
                    "Anyone can view and download games, and we may publish game records in bulk, for example for research or puzzles. Other websites may copy public game records; we have no control over them."
                </p>
            </LegalSection>

            <LegalSection title="Cookies and local storage">
                <p>
                    "We only use cookies that the site needs to work or that remember your choices. We don't use advertising, analytics or third-party cookies, which is why there is no cookie banner."
                </p>
                <LegalList>
                    <li>
                        <strong>"Login session: "</strong>
                        "keeps you signed in for up to 12 weeks."
                    </li>
                    <li>
                        <strong>"Security token: "</strong>
                        "protects forms against cross-site request forgery."
                    </li>
                    <li>
                        <strong>"Preferences: "</strong>
                        "remember your settings, such as theme, board appearance, language, game filters and your last challenge settings."
                    </li>
                    <li>
                        <strong>"Discord linking: "</strong>
                        "a short-lived cookie while you link your Discord account."
                    </li>
                </LegalList>
                <p>
                    "The installable app also uses your browser's local storage, for example to remember that you dismissed a prompt."
                </p>
            </LegalSection>

            <LegalSection title="Who else processes your data">
                <p>
                    "We don't sell or rent your data and we don't share it with advertisers. These service providers process data on our behalf:"
                </p>
                <LegalList>
                    <li>
                        "Hetzner Online GmbH provides the bare-metal servers we run on, in data centres in Germany. We run the operating system, the application and the database ourselves."
                    </li>
                    <li>"Lettermint delivers our account emails."</li>
                    <li>
                        "Your browser's push service delivers notifications, if you turn them on."
                    </li>
                    <li>
                        "Discord delivers notifications to your Discord account, if you link it."
                    </li>
                </LegalList>
                <p>
                    "Some of these providers may process data outside the EU. Where the GDPR requires it, such transfers are covered by safeguards such as the EU-U.S. Data Privacy Framework or standard contractual clauses."
                </p>
            </LegalSection>

            <LegalSection title="How long we keep data">
                <LegalList>
                    <li>
                        "Account data, settings, games and messages: as long as your account exists (see below for what happens when you delete it). Optional profile details are removed when you clear them or delete your account."
                    </li>
                    <li>"Email request records (email and IP address): 24 hours."</li>
                    <li>"Records of sent emails: 30 days."</li>
                    <li>
                        "Password reset links: deleted 7 days after use, or 30 days after they expire."
                    </li>
                    <li>
                        "Browser notification subscriptions: deleted as soon as you turn notifications off on that device. Subscriptions that are switched off another way, by logging out, removing the device on your notifications page or deleting your account, are deleted 90 days later. Subscriptions that go unused for 90 days are deleted too."
                    </li>
                    <li>"Application logs: only as long as we need them to fix problems."</li>
                </LegalList>
            </LegalSection>

            <LegalSection title="Deleting your account">
                <p>
                    "You can delete your account at any time on your account page. We then delete your optional profile details, replace your username and email address with an anonymous placeholder, invalidate your password and switch off your notification subscriptions, which are deleted 90 days later."
                </p>
                <p>
                    "Your past games, rating history and chat messages remain, attributed to the anonymous placeholder, because they are part of other players' games and history. If you linked Discord, contact us to remove that link as well."
                </p>
            </LegalSection>

            <LegalSection title="Your rights">
                <p>
                    "Under the GDPR you have the right to access your data, to have it corrected or deleted, to restrict or object to its processing, and to receive it in a portable format. Where we rely on your consent, you can withdraw it at any time."
                </p>
                <p>
                    "To exercise any of these rights, " <ContactMaintainers />
                    ". You also have the right to lodge a complaint with a data protection supervisory authority."
                </p>
            </LegalSection>

            <LegalSection title="Changes to this policy">
                <p>
                    "If we change this policy, we will update the date at the top of this page and announce significant changes on the site."
                </p>
            </LegalSection>
        </LegalPage>
    }
}
