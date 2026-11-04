use crate::functions::honeytrap::{read_honeytrap, TrapHit};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[component]
pub fn Honeytrap() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let data = Resource::new(move || refresh.get(), |_| read_honeytrap());
    view! {
        <button
            type="button"
            class="ui-button ui-button-secondary ui-button-sm"
            on:click=move |_| refresh.update(|n| *n = n.wrapping_add(1))
        >
            "Refresh"
        </button>
        <Transition fallback=move || {
            view! { <p>"Loading…"</p> }
        }>
            {move || {
                data.get()
                    .map(|result| match result {
                        Err(_) => {
                            view! {
                                <p class="ui-notice">"Could not load crawler observations."</p>
                            }
                                .into_any()
                        }
                        Ok(snapshot) => {
                            let mut users: BTreeMap<String, Vec<TrapHit>> = BTreeMap::new();
                            for hit in snapshot.hits {
                                users.entry(hit.username.clone()).or_default().push(hit);
                            }
                            let mut users: Vec<_> = users.into_iter().collect();
                            users
                                .sort_by_key(|(_, hits)| std::cmp::Reverse(
                                    hits.iter().map(|hit| hit.last_hit).max(),
                                ));
                            view! {
                                <p class="text-sm">
                                    "Guest hits" ": " {snapshot.guest_hits} " (" "Lobby" ": "
                                    {snapshot.guest_lobby_hits} ", " "Account" ": "
                                    {snapshot.guest_account_hits} ")"
                                </p>
                                <div class="overflow-auto max-h-80 text-xs rounded border">
                                    <div class="grid sticky top-0 gap-2 p-2 font-semibold bg-white dark:bg-gray-900 grid-cols-[minmax(8rem,1fr)_4rem_10rem]">
                                        <span>"Username"</span>
                                        <span>"Hits"</span>
                                        <span>"Last hit"</span>
                                    </div>
                                    {users
                                        .into_iter()
                                        .map(|(username, mut hits)| {
                                            hits.sort_by_key(|hit| std::cmp::Reverse(hit.last_hit));
                                            let count = hits
                                                .iter()
                                                .fold(0u64, |count, hit| count.saturating_add(hit.hits));
                                            let last = hits[0]
                                                .last_hit
                                                .format("%Y-%m-%d %H:%M:%S UTC")
                                                .to_string();
                                            view! {
                                                <details class="border-t">
                                                    <summary class="grid gap-2 p-2 cursor-pointer hover:bg-gray-100 grid-cols-[minmax(8rem,1fr)_4rem_10rem] dark:hover:bg-gray-800">
                                                        <span class="font-semibold truncate">{username}</span>
                                                        <span>{count}</span>
                                                        <span>{last}</span>
                                                    </summary>
                                                    <table class="w-full text-left whitespace-nowrap">
                                                        <thead>
                                                            <tr>
                                                                <th class="py-1 px-2">"IP"</th>
                                                                <th class="py-1 px-2">"Page"</th>
                                                                <th class="py-1 px-2">"First hit"</th>
                                                                <th class="py-1 px-2">"Last hit"</th>
                                                                <th class="py-1 px-2">"Hits"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            {hits
                                                                .into_iter()
                                                                .map(|hit| {
                                                                    view! {
                                                                        <tr>
                                                                            <td class="py-1 px-2 font-mono">{hit.ip}</td>
                                                                            <td class="py-1 px-2">
                                                                                {if hit.source == "account" { "Account" } else { "Lobby" }}
                                                                            </td>
                                                                            <td class="py-1 px-2">
                                                                                {hit.first_hit.format("%Y-%m-%d %H:%M:%S UTC").to_string()}
                                                                            </td>
                                                                            <td class="py-1 px-2">
                                                                                {hit.last_hit.format("%Y-%m-%d %H:%M:%S UTC").to_string()}
                                                                            </td>
                                                                            <td class="py-1 px-2">{hit.hits}</td>
                                                                        </tr>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </tbody>
                                                    </table>
                                                </details>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                    })
            }}
        </Transition>
    }
}
