use crate::components::molecules::challenge_row::ChallengeRow;
use crate::functions::{challenges::get::get_challenge, hostname::hostname_and_port};
use crate::providers::AuthContext;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};
use shared_types::ChallengeId;

#[derive(Params, PartialEq, Eq)]
struct ChallengeParams {
    nanoid: String,
}

#[component]
pub fn ChallengeView() -> impl IntoView {
    let params = use_params::<ChallengeParams>();
    let auth_context = expect_context::<AuthContext>();
    let nanoid = Signal::derive(move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    });
    let challenge = OnceResource::new(get_challenge(ChallengeId(nanoid.get_untracked())));

    let challenge_address = move || format!("{}/challenge/{}", hostname_and_port(), nanoid());

    let copy_state = RwSignal::new(false);

    let interval = StoredValue::new(use_interval_fn_with_options(
        move || copy_state.set(false),
        2000,
        UseIntervalFnOptions::default().immediate(false),
    ));

    let copy = move |_| {
        let interval = interval.get_value();
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist in challenge_view")
            .navigator()
            .clipboard();
        let _ = clipboard.write_text(&challenge_address());
        copy_state.set(true);
        (interval.pause)();
        (interval.resume)();
    };

    let copy_button_class = move || {
        let base_classes = "px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform active:scale-95 focus:outline-none focus:shadow-outline";
        if copy_state.get() {
            format!("{base_classes} bg-grasshopper-green hover:bg-green-500")
        } else {
            format!("{base_classes} bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal")
        }
    };
    let uid = move || auth_context.user.with(|a| a.as_ref().map(|user| user.id));
    view! {
        <div class="flex flex-col items-center pt-20 mx-auto">
            <Suspense fallback=move || {
                view! {
                    <div class="flex justify-center items-center p-8">
                        <div class="text-center">
                            <div class="mb-2 text-lg">"Loading challenge..."</div>
                            <div class="text-sm opacity-75">
                                "Please wait while we fetch the challenge details"
                            </div>
                        </div>
                    </div>
                }
            }>
                <ErrorBoundary fallback=|_errors| {
                    view! {
                        <div class="flex justify-center items-center p-8">
                            <div class="text-center text-red-500">
                                <div class="mb-2 text-lg">"Error loading challenge"</div>
                                <div class="text-sm">"Challenge doesn't seem to exist"</div>
                            </div>
                        </div>
                    }
                }>
                    {move || {
                        challenge
                            .get()
                            .map(|data| match data {
                                Err(_) => {
                                    Either::Left(
                                        view! {
                                            <div class="flex justify-center items-center p-8">
                                                <div class="text-center text-red-500">
                                                    <div class="mb-2 text-lg">"Challenge not found"</div>
                                                    <div class="text-sm">
                                                        "The challenge you're looking for doesn't exist"
                                                    </div>
                                                </div>
                                            </div>
                                        },
                                    )
                                }
                                Ok(challenge) => {
                                    let user = auth_context.user;
                                    Either::Right(
                                        view! {
                                            <Show when=move || {
                                                user.with(|a| {
                                                    a.as_ref()
                                                        .is_some_and(|user| user.id == challenge.challenger.uid)
                                                })
                                            }>
                                                <p>"To invite someone to play, give this URL:"</p>
                                                <div class="flex">
                                                    <input
                                                        id="challenge_link"
                                                        type="text"
                                                        class="w-[50ch]"
                                                        value=challenge_address
                                                        readonly
                                                    />
                                                    <button
                                                        title="Copy link"
                                                        on:click=copy
                                                        class=copy_button_class
                                                    >
                                                        <Icon icon=icondata_ai::AiCopyOutlined attr:class="w-6 h-6" />
                                                    </button>
                                                </div>
                                                <p>
                                                    "The first person to come to this URL will play with you."
                                                </p>
                                            </Show>
                                            <ChallengeRow challenge=challenge single=true uid=uid() />
                                        },
                                    )
                                }
                            })
                    }}
                </ErrorBoundary>
            </Suspense>
        </div>
    }
}
