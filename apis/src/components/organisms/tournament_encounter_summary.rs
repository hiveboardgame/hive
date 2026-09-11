use crate::components::atoms::color_hex::ColorHex;
use hive_lib::Color;
use leptos::prelude::*;
use leptos_icons::Icon;

#[derive(Clone)]
pub struct EncounterParticipant {
    pub(crate) name: String,
    pub(crate) detail: Option<String>,
    pub(crate) color: Option<Color>,
}

#[component]
fn EncounterParticipantLine(participant: EncounterParticipant) -> impl IntoView {
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
                                {detail}
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
    #[prop(optional)] openable: bool,
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
        <div class="grid gap-2 items-center h-20 @min-[30rem]:h-12 grid-cols-[1.5rem_minmax(0,1fr)_minmax(3rem,6rem)_0.75rem]">
            <span class="text-xs font-bold tabular-nums text-center text-gray-500">{identity}</span>
            <div class="space-y-1.5 min-w-0">
                <EncounterParticipantLine participant=first />
                {second.map(|participant| view! { <EncounterParticipantLine participant /> })}
            </div>
            <div class="text-right">
                <strong class="block text-sm tabular-nums leading-tight wrap-break-word">
                    {result}
                </strong>
            </div>
            <Show when=move || openable>
                <Icon
                    icon=icondata_lu::LuChevronRight
                    attr:class="size-3 text-gray-500 dark:text-gray-400"
                />
            </Show>
        </div>
    }
}
