use crate::components::atoms::profile_link::ProfileLink;
use crate::responses::UserResponse;
use leptos::prelude::*;

pub enum TableRowLabel {
    Text(String),
    User(UserResponse),
}

pub struct TableRow {
    pub label: TableRowLabel,
    pub values: Vec<i64>,
    pub additional_values: Option<Vec<f64>>,
}

#[component]
pub fn StatTableCardGeneric(
    #[prop(optional)] title: Option<&'static str>,
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
                            let label_view = match row.label {
                                TableRowLabel::Text(text) => {

                                    view! { <span>{text}</span> }
                                        .into_any()
                                }
                                TableRowLabel::User(user) => {
                                    view! {
                                        <ProfileLink
                                            username=user.username.clone()
                                            patreon=user.patreon
                                            bot=user.bot
                                            user_is_hoverable=Some(user).into()
                                            use_default_style=false
                                        />
                                    }
                                        .into_any()
                                }
                            };

                            view! {
                                <tr class="border-b border-gray-200 last:border-none">
                                    <td class="py-1">{label_view}</td>
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
