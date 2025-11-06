use leptos::prelude::*;

#[component]
pub fn SingleStatCard(
    label: &'static str,
    value: i64,
    #[prop(optional)] additional_value: Option<f64>,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-700 rounded-2xl shadow p-4">
            <div class="text-2xl font-bold">{value.to_string()}</div>
            {additional_value
                .map(|add_val| {
                    view! {
                        <div class="text-m text-gray-500 dark:text-gray-300 ml-1">
                            {format!("{add_val:.1}%")}
                        </div>
                    }
                })}
            <div class="text-sm text-gray-500 dark:text-gray-300">{label}</div>
        </div>
    }
}
