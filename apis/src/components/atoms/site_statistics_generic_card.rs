use leptos::prelude::*;
use crate::components::atoms::profile_link::ProfileLink;
use crate::functions::users::get_user_by_username;

pub struct TableRow {
    pub label: String,
    pub values: Vec<i64>,
    pub additional_values: Option<Vec<f64>>,
}

#[component]
pub fn StatTableCardGeneric(
    #[prop(optional)] title: Option<&'static str>,
    #[prop(optional)] first_column_is_username: bool,
    headers: Vec<&'static str>,
    rows: Vec<TableRow>,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-700 rounded-2xl shadow p-4">
            <Show when=move || title.is_some()>
                <h3 class="font-semibold mb-2">{title.unwrap()}</h3>
            </Show>
            <table class="w-full text-sm">
                <thead class="border-b border-gray-300">
                    <tr>
                        {headers
                            .into_iter()
                            .map(|h| {
                                view! { <th class="py-1">{h}</th> }
                            })
                            .collect::<Vec<_>>()}
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|row| {
                            let additional_values = row.additional_values.clone();
                            let label = row.label.clone();

                            view! {
                                <tr class="border-b border-gray-200 last:border-none">
                                    <td class="py-1">
                                        {if first_column_is_username {
                                            let username_for_resource = label.clone();
                                            let label_for_fallback = label.clone();
                                            let label_for_error = label.clone();
                                            
                                            let user_resource = LocalResource::new(move || {
                                                get_user_by_username(username_for_resource.clone())
                                            });

                                            view! {
                                                <Suspense fallback=move || {
                                                    view! { <span>{label_for_fallback.clone()}</span> }
                                                }>
                                                    {move || {
                                                        user_resource
                                                            .get()
                                                            .map(|user_result| {
                                                                match user_result {
                                                                    Ok(user) => {
                                                                        let user_is_hoverable = Some(user.clone());
                                                                        view! {
                                                                            <ProfileLink
                                                                                username=user.username.clone()
                                                                                patreon=user.patreon
                                                                                bot=user.bot
                                                                                user_is_hoverable=user_is_hoverable.into()
                                                                            />
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        view! {
                                                                            <ProfileLink
                                                                                username=label_for_error.clone()
                                                                                patreon=false
                                                                                bot=false
                                                                            />
                                                                        }
                                                                    }
                                                                }
                                                            })
                                                    }}

                                                </Suspense>
                                            }
                                                .into_any()
                                        } else {
                                            view! { <span>{label}</span> }.into_any()
                                        }}

                                    </td>
                                    {row
                                        .values
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, value)| {
                                            let additional = additional_values
                                                .as_ref()
                                                .and_then(|av| av.get(i))
                                                .copied();

                                            view! {
                                                <td class="py-1 text-center">
                                                    <div>{value.to_string()}</div>
                                                    <Show when=move || additional.is_some()>
                                                        <div class="text-xs text-gray-500 dark:text-gray-300">
                                                            {format!("{:.1}", additional.unwrap())} "%"
                                                        </div>
                                                    </Show>
                                                </td>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </tr>
                            }
                        })
                        .collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    }
}