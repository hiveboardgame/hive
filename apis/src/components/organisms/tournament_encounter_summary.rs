use crate::{
    components::atoms::{color_hex::ColorHex, rating::Rating},
    i18n::*,
};
use hive_lib::Color;
use leptos::prelude::*;

#[derive(Clone)]
pub struct EncounterParticipant {
    pub(crate) name: String,
    pub(crate) detail: Option<String>,
    pub(crate) rating: Option<u64>,
    pub(crate) color: Option<Color>,
}

#[component]
fn EncounterParticipantLine(participant: EncounterParticipant) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="grid gap-2 items-center min-w-0 grid-cols-[1rem_minmax(0,1fr)]">
            <span class="inline-flex justify-center">
                {participant
                    .color
                    .map(|color| view! { <ColorHex color=Signal::derive(move || color) /> })}
            </span>
            <div class="flex flex-col gap-x-2 min-w-0 @min-[30rem]:flex-row @min-[30rem]:items-baseline">
                <span class="font-bold truncate" title=participant.name.clone()>
                    {participant.name.clone()}
                </span>
                {participant
                    .detail
                    .map(|detail| {
                        view! {
                            <span class="text-xs tabular-nums text-gray-500 shrink-0">
                                {t!(
                                    i18n, tournaments.ui.pts, value1 = move || view! { <Rating value=participant.rating /> }, value2 = detail
                                )}
                            </span>
                        }
                    })}
            </div>
        </div>
    }
}

#[component]
pub(crate) fn TournamentEncounterSummary(
    identity: String,
    left: EncounterParticipant,
    right: Option<EncounterParticipant>,
    result: String,
    #[prop(optional)] progress: Option<String>,
    #[prop(optional, into)] action: Option<TextProp>,
    #[prop(optional)] game_count: Option<usize>,
) -> impl IntoView {
    let (first, second) = if left.color == Some(Color::Black)
        && right
            .as_ref()
            .is_some_and(|participant| participant.color == Some(Color::White))
    {
        (right.expect("white participant exists"), Some(left))
    } else {
        (left, right)
    };
    view! {
        <div class="grid gap-y-1 gap-x-2 items-center min-h-20 @min-[30rem]:min-h-16 grid-cols-[1.5rem_minmax(0,1fr)_auto]">
            <span class="text-xs font-bold tabular-nums text-center text-gray-500 @min-[30rem]:row-span-2">
                {identity}
            </span>
            <div class="space-y-1.5 min-w-0 @min-[30rem]:row-span-2">
                <EncounterParticipantLine participant=first />
                {second.map(|participant| view! { <EncounterParticipantLine participant /> })}
            </div>
            <div class="text-right">
                <strong class="block text-sm tabular-nums leading-tight whitespace-nowrap">
                    {result}
                </strong>
            </div>
            <div class="flex flex-wrap col-span-2 col-start-2 gap-y-1 gap-x-3 justify-between items-center text-xs @min-[30rem]:col-span-1 @min-[30rem]:col-start-3 @min-[30rem]:flex-col @min-[30rem]:items-end">
                {progress.map(|progress| view! { <span class="text-gray-500">{progress}</span> })}
                {action
                    .map(|action| {
                        view! {
                            <span class="flex gap-2 items-baseline ml-auto whitespace-nowrap">

                                {game_count
                                    .map(|count| {
                                        view! {
                                            <span class="text-gray-500">
                                                {move || {
                                                    t_string!(use_i18n(), profile.games_played, count = count)
                                                        .to_string()
                                                }}
                                            </span>
                                        }
                                    })}
                                <span class="font-semibold underline underline-offset-2">
                                    {move || action.get()}
                                </span>
                            </span>
                        }
                    })}
            </div>
        </div>
    }
}
