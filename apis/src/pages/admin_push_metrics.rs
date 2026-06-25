use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{empty_state::EmptyState, panel::Panel},
    },
    functions::push_metrics::read_push_metrics,
    providers::AuthContext,
};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use shared_types::PushMetrics;

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
        <PageShell variant=PageShellVariant::Dashboard>
            <Show
                when=move || {
                    auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
                }
                fallback=|| {
                    view! {
                        <EmptyState
                            title="Push metrics unavailable"
                            message="This page is only available to administrators."
                        />
                    }
                }
            >
                <div class="flex flex-col gap-4 w-full max-w-5xl">
                    <PageHeader
                        title="Push Notification Metrics"
                        subtitle="Delivery, retry, and suppression counters since server start."
                    />
                    <div class="flex flex-wrap gap-2 items-center">
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| refresh_token.update(|n| *n = n.wrapping_add(1))
                        >
                            "Refresh"
                        </button>
                        <span class="text-xs text-gray-600 dark:text-gray-400">
                            "Auto-refresh every 5s"
                        </span>
                    </div>
                    <Suspense fallback=move || {
                        view! { <EmptyState title="Loading push metrics..." /> }
                    }>
                        {move || {
                            data.get()
                                .map(|res| match res {
                                    Ok(m) => view! { <Tables m=m /> }.into_any(),
                                    Err(e) => {
                                        view! {
                                            <EmptyState
                                                title="Push metrics error"
                                                message=format!("{e}")
                                            />
                                        }
                                            .into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </Show>
        </PageShell>
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
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <Panel title="Intake">
                <Table>
                    <Row label="received" value=m.received />
                    <Row label="dropped (queue full)" value=m.dropped_queue_full alert=true />
                    <Row label="processed" value=processed muted=true />
                </Table>
            </Panel>

            <Panel title="Per-event disposition">
                <Table>
                    <Row label="suppressed by prefs" value=m.suppressed_prefs />
                    <Row label="prefs DB error" value=m.prefs_db_error alert=true />
                    <Row label="ack-eligible (your-turn)" value=m.ack_eligible muted=true />
                    <Row label="ack-suppressed (watching)" value=m.ack_suppressed />
                    <Row label="ack-fired (sent after park)" value=m.ack_fired />
                    <Row label="test pushes" value=m.test_pushes />
                </Table>
            </Panel>

            <Panel title="Per-device send outcomes">
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
            </Panel>

            <Panel title="Derived">
                <Table>
                    <RateRow label="send attempts" value=attempts.to_string() />
                    <RateRow label="delivery rate" value=delivery_rate />
                    <RateRow label="ack-suppression rate" value=ack_rate />
                </Table>
            </Panel>
        </div>
    }
}

#[component]
fn Table(children: Children) -> impl IntoView {
    view! {
        <table class="w-full text-sm text-gray-900 border-collapse dark:text-gray-100">
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
        "py-1 font-mono font-bold text-right text-ladybug-red"
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
        <tr class="border-b last:border-b-0 border-black/10 dark:border-white/10">
            <td class=label_class>{label}</td>
            <td class=value_class>{value}</td>
        </tr>
    }
}

#[component]
fn RateRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <tr class="border-b last:border-b-0 border-black/10 dark:border-white/10">
            <td class="py-1">{label}</td>
            <td class="py-1 font-mono text-right">{value}</td>
        </tr>
    }
}
