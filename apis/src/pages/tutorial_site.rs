use crate::{
    common::TimeParamsStoreFields,
    components::layouts::page_shell::PageShell,
    pages::{
        challenge_create::ChallengeCreate,
        config::Config,
        home::Home,
        tournament_create::TournamentCreate,
        tournaments::{Tournaments, TournamentsByStatus},
    },
    providers::{ChallengeParams, ChallengeParamsStoreFields},
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use reactive_stores::Store;
use shared_types::{TimeMode, TournamentStatus};

/// The real page components, rendered in a script-free, read-only lesson frame.
/// The iframe sandbox blocks scripts and form submission. Disabled form controls
/// also prevent changes when this route is opened directly.
#[component]
pub fn TutorialSitePage() -> impl IntoView {
    let query = use_query_map();
    let screen = query.with_untracked(|query| query.get("screen")).unwrap_or_default();
    let focus = query.with_untracked(|query| query.get("focus")).unwrap_or_default();
    if screen == "correspondence" {
        let params = expect_context::<Store<ChallengeParams>>();
        params.time_signals().time_mode().set(TimeMode::Correspondence);
    }
    let content = match screen.as_str() {
        "settings" => view! { <Config /> }.into_any(),
        "tournaments" => view! {
            <Tournaments><TournamentsByStatus status=TournamentStatus::NotStarted /></Tournaments>
        }.into_any(),
        "tournament-create" => view! { <TournamentCreate /> }.into_any(),
        "correspondence" => view! { <PageShell><ChallengeCreate /></PageShell> }.into_any(),
        _ => view! { <TutorialHomepage /> }.into_any(),
    };
    let target = match focus.as_str() {
        "custom" => ("#tutorial-custom-game", "➜ Create your own game"),
        "challenges" => ("#tutorial-open-challenges", "➜ Join an open game here"),
        "feedback" => ("#home-feedback", "➜ Share your feedback here"),
        "quickplay" => ("#tutorial-quickplay", "➜ Choose a time control"),
        _ => ("#tutorial-no-focus", ""),
    };
    let css = format!(
        "main.ui-app-background {{ display: block !important; }}
         main > div.absolute {{ display: none !important; }}
         a, button, input, select, textarea, summary, video {{ pointer-events: none !important; }}
         html {{ scroll-padding-top: 65px; }}
         fieldset[aria-label='Read-only actual site page'] :disabled {{ opacity: 1 !important; }}
         {} {{ position: relative; outline: 2px solid #3f9b3a; outline-offset: 3px; scroll-margin-top: 65px; }}
         {}::before {{ content: '{}'; position: absolute; left: 0; top: 0; transform: translateY(-100%); z-index: 10; color: #277423; background: #fff; padding: 3px 6px; font: bold 12px sans-serif; }}",
        target.0, target.0, target.1,
    );
    view! {
        <style>{css}</style>
        <fieldset disabled class="contents" aria-label="Read-only actual site page">{content}</fieldset>
    }
}

#[component]
fn TutorialHomepage() -> impl IntoView {
    let challenges = OnceResource::new(async {
        #[cfg(feature = "ssr")]
        {
            use crate::{functions::db::pool, responses::ChallengeResponse};
            use db_lib::{get_conn, models::Challenge};
            let loaded = async {
                let pool = pool().await?;
                let mut conn = get_conn(&pool).await?;
                let models = Challenge::get_public(&mut conn).await?;
                ChallengeResponse::from_models_batch(models, &mut conn).await
                    .map_err(ServerFnError::new)
            }.await;
            match loaded {
                Ok(challenges) => return challenges,
                Err(error) => log::warn!("tutorial homepage challenges: {error}"),
            }
        }
        Vec::<crate::responses::ChallengeResponse>::new()
    });
    let challenge_state = expect_context::<crate::providers::challenges::ChallengeStateSignal>();
    view! {
        <Suspense fallback=|| view! { <p>"Loading homepage…"</p> }>
            {move || challenges.get().map(|challenges| {
                let mut challenge_state = challenge_state;
                challenge_state.snapshot_apply(challenges);
                view! { <Home /> }
            })}
        </Suspense>
    }
}
