use leptos::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">
                Fair Play Cup 2025 - Team Tournament
            </h1>
            <div class="flex flex-col">
                <a
                    href="https://www.worldhivetournaments.com/hive-fair-play-cup/"
                    rel="external"
                    target="_blank"
                    class="place-self-center text-blue-500 hover:underline"
                >
                    Sign up by January 4th 2025
                </a>
                <ul>
                    <li>Starts January 6th</li>
                    <li>
                        Each team consists of 3 to 4 players (only 3 players per team play each round)
                    </li>
                    <li>One match (2 games) every 2 weeks</li>
                    <li>Time control: 15 min + 15 sec</li>
                </ul>
                <div>
                    Schedule your games here or over
                    <a
                        href="https://discord.gg/YwDEmYPHrZ"
                        rel="external"
                        target="_blank"
                        class="text-blue-500 hover:underline"
                    >
                        Discord
                    </a>
                </div>
            </div>
        </div>
    }
}
