use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{empty_state::EmptyState, panel::Panel},
    },
    functions::push_metrics::read_push_metrics,
    i18n::{t, t_string, use_i18n},
    providers::AuthContext,
};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use shared_types::PushMetrics;

#[component]
pub fn AdminPushMetrics() -> impl IntoView {
    let i18n = use_i18n();
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
                fallback=move || {
                    view! {
                        <EmptyState
                            title=move || {
                                t_string!(i18n, common.admin.push_metrics.push_metrics_unavailable)
                            }
                            message=move || t_string!(i18n, common.admin.administrators_only)
                        />
                    }
                }
            >
                <div class="flex flex-col gap-4 w-full max-w-5xl">
                    <PageHeader
                        title=move || {
                            t_string!(i18n, common.admin.push_metrics.push_notification_metrics)
                        }
                        subtitle=move || {
                            t_string!(
                                i18n, common.admin.push_metrics.delivery_retry_and_suppression_counters_since_server_start
                            )
                        }
                    />
                    <div class="flex flex-wrap gap-2 items-center">
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| refresh_token.update(|n| *n = n.wrapping_add(1))
                        >
                            {t!(i18n, common.refresh)}
                        </button>
                        <span class="text-xs text-gray-600 dark:text-gray-400">
                            {t!(i18n, common.admin.push_metrics.auto_refresh_every_5s)}
                        </span>
                    </div>
                    <Suspense fallback=move || {
                        view! { <EmptyState title=move || t_string!(i18n, common.loading) /> }
                    }>
                        {move || {
                            data.get()
                                .map(|res| match res {
                                    Ok(m) => view! { <Tables m=m /> }.into_any(),
                                    Err(e) => {
                                        view! {
                                            <EmptyState
                                                title=move || {
                                                    t_string!(
                                                        i18n, common.admin.push_metrics.push_metrics_error
                                                    )
                                                }
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
    let i18n = use_i18n();
    let processed = m.received.saturating_sub(m.dropped_queue_full);
    let attempts = m.delivered + m.retryable + m.token_dead + m.failed;
    let delivery_rate = pct(m.delivered, m.delivered + m.token_dead + m.failed);
    let ack_rate = pct(m.ack_suppressed, m.ack_eligible);

    view! {
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <Panel title=move || t_string!(i18n, common.admin.push_metrics.intake)>
                <Table>
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.received)
                        value=m.received
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.dropped_queue_full)
                        value=m.dropped_queue_full
                        alert=true
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.processed)
                        value=processed
                        muted=true
                    />
                </Table>
            </Panel>

            <Panel title=move || t_string!(i18n, common.admin.push_metrics.per_event_disposition)>
                <Table>
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.suppressed_by_prefs)
                        value=m.suppressed_prefs
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.prefs_db_error)
                        value=m.prefs_db_error
                        alert=true
                    />
                    <Row
                        label=move || {
                            t_string!(i18n, common.admin.push_metrics.ack_eligible_your_turn)
                        }
                        value=m.ack_eligible
                        muted=true
                    />
                    <Row
                        label=move || {
                            t_string!(i18n, common.admin.push_metrics.ack_suppressed_watching)
                        }
                        value=m.ack_suppressed
                    />
                    <Row
                        label=move || {
                            t_string!(i18n, common.admin.push_metrics.ack_fired_sent_after_park)
                        }
                        value=m.ack_fired
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.test_pushes)
                        value=m.test_pushes
                    />
                </Table>
            </Panel>

            <Panel title=move || {
                t_string!(i18n, common.admin.push_metrics.per_device_send_outcomes)
            }>
                <Table>
                    <Row
                        label=move || {
                            t_string!(i18n, common.admin.push_metrics.no_registered_device)
                        }
                        value=m.no_device
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.device_db_error)
                        value=m.device_db_error
                        alert=true
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.delivered)
                        value=m.delivered
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.retryable)
                        value=m.retryable
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.token_dead_reaped)
                        value=m.token_dead
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.failed)
                        value=m.failed
                        alert=true
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.retry_delivered)
                        value=m.retry_delivered
                    />
                    <Row
                        label=move || t_string!(i18n, common.admin.push_metrics.retry_gave_up)
                        value=m.retry_gave_up
                        alert=true
                    />
                </Table>
            </Panel>

            <Panel title=move || t_string!(i18n, common.admin.push_metrics.derived)>
                <Table>
                    <RateRow
                        label=move || t_string!(i18n, common.admin.push_metrics.send_attempts)
                        value=attempts.to_string()
                    />
                    <RateRow
                        label=move || t_string!(i18n, common.admin.push_metrics.delivery_rate)
                        value=delivery_rate
                    />
                    <RateRow
                        label=move || {
                            t_string!(i18n, common.admin.push_metrics.ack_suppression_rate)
                        }
                        value=ack_rate
                    />
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
    #[prop(into)] label: TextProp,
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
            <td class=label_class>{move || label.get()}</td>
            <td class=value_class>{value}</td>
        </tr>
    }
}

#[component]
fn RateRow(#[prop(into)] label: TextProp, value: String) -> impl IntoView {
    view! {
        <tr class="border-b last:border-b-0 border-black/10 dark:border-white/10">
            <td class="py-1">{move || label.get()}</td>
            <td class="py-1 font-mono text-right">{value}</td>
        </tr>
    }
}
