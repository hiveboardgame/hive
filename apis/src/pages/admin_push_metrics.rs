use crate::{functions::push_metrics::read_push_metrics, providers::AuthContext};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use shared_types::PushMetrics;

const SECTION_TITLE: &str = "pt-4 pb-1 text-lg font-bold dark:text-white";

#[component]
pub fn AdminPushMetrics() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let refresh_token = RwSignal::new(0u32);
    let data = Resource::new(move || refresh_token.get(), |_| read_push_metrics());

    use_interval_fn(
        move || refresh_token.update(|n| *n = n.wrapping_add(1)),
        5_000,
    );

    view! {
        <div class="px-4 pt-page">
            <Show when=move || {
                auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
            }>
                <h1 class="pb-2 text-2xl font-bold dark:text-white">"Push Notification Metrics"</h1>
                <div class="flex flex-wrap gap-2 items-center pb-3">
                    <button
                        class="py-1 px-3 text-sm font-semibold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                        on:click=move |_| refresh_token.update(|n| *n = n.wrapping_add(1))
                    >
                        "Refresh"
                    </button>
                    <span class="text-xs text-gray-600 dark:text-gray-400">
                        "Auto-refresh every 5s · cumulative since server start"
                    </span>
                </div>
                <Suspense fallback=move || {
                    view! { <p class="dark:text-white">"Loading…"</p> }
                }>
                    {move || {
                        data.get()
                            .map(|res| match res {
                                Ok(m) => view! { <Tables m=m /> }.into_any(),
                                Err(e) => {
                                    view! {
                                        <p class="text-red-500">
                                            "Push metrics error: " {format!("{e}")}
                                        </p>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </Show>
        </div>
    }
}

fn pct(num: u64, den: u64) -> String {
    if den == 0 {
        "—".to_string()
    } else {
        format!("{:.1}%", num as f64 / den as f64 * 100.0)
    }
}

#[component]
fn Tables(m: PushMetrics) -> impl IntoView {
    let processed = m.received.saturating_sub(m.dropped_queue_full);
    let attempts = m.delivered + m.retryable + m.token_dead + m.failed;
    let delivery_rate = pct(m.delivered, m.delivered + m.token_dead + m.failed);
    let ack_rate = pct(m.ack_suppressed, m.ack_eligible);

    view! {
        <div class="grid grid-cols-1 gap-x-8 max-w-4xl lg:grid-cols-2">
            <div>
                <h2 class=SECTION_TITLE>"Intake"</h2>
                <Table>
                    <Row label="received" value=m.received />
                    <Row label="dropped (queue full)" value=m.dropped_queue_full alert=true />
                    <Row label="processed" value=processed muted=true />
                </Table>

                <h2 class=SECTION_TITLE>"Per-event disposition"</h2>
                <Table>
                    <Row label="suppressed by prefs" value=m.suppressed_prefs />
                    <Row label="prefs DB error" value=m.prefs_db_error alert=true />
                    <Row label="ack-eligible (your-turn)" value=m.ack_eligible muted=true />
                    <Row label="ack-suppressed (watching)" value=m.ack_suppressed />
                    <Row label="ack-fired (sent after park)" value=m.ack_fired />
                    <Row label="test pushes" value=m.test_pushes />
                </Table>
            </div>

            <div>
                <h2 class=SECTION_TITLE>"Per-device send outcomes"</h2>
                <Table>
                    <Row label="no registered device" value=m.no_device />
                    <Row label="device DB error" value=m.device_db_error alert=true />
                    <Row label="delivered" value=m.delivered />
                    <Row label="retryable" value=m.retryable />
                    <Row label="token dead (reaped)" value=m.token_dead />
                    <Row label="failed" value=m.failed alert=true />
                    <Row label="retry delivered" value=m.retry_delivered />
                    <Row label="retry gave up" value=m.retry_gave_up alert=true />
                </Table>

                <h2 class=SECTION_TITLE>"Derived"</h2>
                <Table>
                    <RateRow label="send attempts" value=attempts.to_string() />
                    <RateRow label="delivery rate" value=delivery_rate />
                    <RateRow label="ack-suppression rate" value=ack_rate />
                </Table>
            </div>
        </div>
    }
}

#[component]
fn Table(children: Children) -> impl IntoView {
    view! {
        <table class="w-full text-sm border-collapse dark:text-white">
            <tbody>{children()}</tbody>
        </table>
    }
}

#[component]
fn Row(
    label: &'static str,
    value: u64,
    #[prop(optional)] alert: bool,
    #[prop(optional)] muted: bool,
) -> impl IntoView {
    let value_class = if alert && value > 0 {
        "py-1 font-mono font-bold text-right text-red-500"
    } else if muted {
        "py-1 font-mono text-right text-gray-500 dark:text-gray-400"
    } else {
        "py-1 font-mono text-right"
    };
    let label_class = if muted {
        "py-1 text-gray-500 dark:text-gray-400"
    } else {
        "py-1"
    };
    view! {
        <tr class="border-b border-gray-200 dark:border-gray-700">
            <td class=label_class>{label}</td>
            <td class=value_class>{value}</td>
        </tr>
    }
}

#[component]
fn RateRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <tr class="border-b border-gray-200 dark:border-gray-700">
            <td class="py-1">{label}</td>
            <td class="py-1 font-mono text-right">{value}</td>
        </tr>
    }
}
