use leptos::prelude::*;

#[component]
pub fn RlBanner() -> impl IntoView {
    view! {
        <div class="flex flex-col justify-center items-center p-6 mb-3 text-black rounded-sm bg-orange-twilight xs:p-8 xs:mb-4">
            <h1 class="flex items-center mb-4 text-2xl font-bold xs:text-4xl">Hive Rapid League</h1>
            <p>Season 7 starts soon!</p>
            <p>Play against opponents at a similar level</p>
            <p>10 weeks, 1 game per week, flexible scheduling</p>
            <p>Time control is 10 min + 10 sec</p>
            <p>
                <a
                    href="https://forms.gle/kiAHxJoxQw5DJQvw7"
                    rel="external"
                    target="_blank"
                    class="place-self-center text-blue-500 hover:underline inline"
                >
                    Sign up
                </a>
                by 15th February
            </p>
        </div>
    }
}
