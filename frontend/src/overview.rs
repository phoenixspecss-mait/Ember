use crate::api::{Alert, AlertsResponse, Reading};
use crate::charts::{LineChart, ScoreRing};
use crate::fmt::human_num;
use crate::state::Store;
use crate::tree::CampusTree;
use crate::ui::{metric_label, CountUp, Delta, SectionHead, Tag};
use leptos::*;

fn campus_series(readings: &[Reading], alerts: &AlertsResponse) -> (Vec<(String, f64)>, Vec<usize>) {
    let mut dates: Vec<String> = readings.iter().map(|r| r.date.clone()).collect();
    dates.sort();
    dates.dedup();
    let mut series: Vec<(String, f64)> = Vec::with_capacity(dates.len());
    for d in &dates {
        let sum: f64 = readings.iter().filter(|r| &r.date == d).map(|r| r.kwh).sum();
        series.push((d.clone(), sum));
    }
    let mut anom: Vec<usize> = alerts
        .alerts
        .iter()
        .filter(|a| a.metric == "kwh")
        .filter_map(|a| dates.iter().position(|d| d == &a.date))
        .collect();
    anom.sort_unstable();
    anom.dedup();
    (series, anom)
}

#[component]
pub fn Overview() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");

    let co2 = create_memo(move |_| store.summary.get().co2_kg);
    let co2_delta = create_memo(move |_| store.summary.get().change_pct.co2_kg);
    let kwh = create_memo(move |_| store.summary.get().kwh);
    let kwh_delta = create_memo(move |_| store.summary.get().change_pct.kwh);
    let water = create_memo(move |_| store.summary.get().water_l);
    let water_delta = create_memo(move |_| store.summary.get().change_pct.water_l);
    let waste = create_memo(move |_| store.summary.get().waste_kg);
    let waste_delta = create_memo(move |_| store.summary.get().change_pct.waste_kg);
    let score = create_memo(move |_| store.summary.get().campus_score);
    let window_end = create_memo(move |_| store.summary.get().window_end);

    let series = create_memo(move |_| {
        let (r, a) = (store.readings.get(), store.alerts.get());
        campus_series(&r, &a).0
    });
    let anom = create_memo(move |_| {
        let (r, a) = (store.readings.get(), store.alerts.get());
        campus_series(&r, &a).1
    });
    let chart_aria = create_memo(move |_| {
        let (r, a) = (store.readings.get(), store.alerts.get());
        let (s, _) = campus_series(&r, &a);
        let avg = if s.is_empty() {
            0.0
        } else {
            s.iter().map(|p| p.1).sum::<f64>() / s.len() as f64
        };
        format!(
            "Campus electricity over the last 60 days, averaging {} kilowatt hours per day. Vermillion markers mark anomalous days.",
            human_num(avg, 0)
        )
    });

    let alerts = create_memo(move |_| store.alerts.get().alerts);
    let alert_count = create_memo(move |_| store.alerts.get().count);
    let suggestions = create_memo(move |_| store.suggestions.get().suggestions);

    view! {
        <div class="overview">
            <section class="hero" aria-label="Carbon this week">
                <span class="giant-num" aria-hidden="true">01</span>
                <div class="hero-main">
                    <p class="kicker rise">
                        <span class="mask"><span class="mask-inner">
                            "CO2 this week · seven days ending " {move || window_end.get()}
                        </span></span>
                    </p>
                    <h1 class="hero-number">
                        <span class="mask">
                            <span class="mask-inner">
                                <CountUp value=co2 class="hero-count"/>
                            </span>
                        </span>
                        <span class="hero-unit rise">"kg"</span>
                    </h1>
                    <p class="hero-sub rise">
                        <span class="mask"><span class="mask-inner">
                            <Delta value=co2_delta/>
                            "% week-over-week · 0.82 kg CO2 per kWh burned"
                        </span></span>
                    </p>
                </div>
                <div class="hero-side">
                    <CampusTree score=score/>
                    <div class="hero-ring">
                        <ScoreRing score=score size=118.0 caption="campus score"/>
                    </div>
                </div>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead
                    num="01"
                    title="Electricity, campus-wide"
                    note="six buildings · trailing 60 days · kWh per day"
                />
                <LineChart points=series anomalies=Some(anom) unit=" kWh" aria=chart_aria/>
                <p class="chart-note">
                    Hostels carry the night-time AC signature: spiky weeks, higher weekends.
                    Library and the academic block run flat and drop hard on Sundays.
                </p>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead
                    num="02"
                    title="The week in three meters"
                    note="totals and change against the previous seven days"
                />
                <div class="stat-strip">
                    <div class="stat">
                        <span class="stat-label">electricity</span>
                        <span class="stat-value">
                            <CountUp value=kwh decimals=0 suffix=" kWh"/>
                        </span>
                        <span class="stat-delta"><Delta value=kwh_delta/>% on last week</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">water</span>
                        <span class="stat-value">
                            <CountUp value=water decimals=0 suffix=" L"/>
                        </span>
                        <span class="stat-delta"><Delta value=water_delta/>% on last week</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">waste</span>
                        <span class="stat-value">
                            <CountUp value=waste decimals=0 suffix=" kg"/>
                        </span>
                        <span class="stat-delta"><Delta value=waste_delta/>% on last week</span>
                    </div>
                </div>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section grid-2">
                <div>
                    <SectionHead
                        num="03"
                        title="Anomalies on file"
                        note="flagged by the rolling 7-day rule"
                    />
                    <div class="alerts" role="list" aria-label="Anomalies">
                        {move || {
                            let list = alerts.get();
                            if list.is_empty() {
                                view! {
                                    <p class="empty-note">
                                        "No anomalies in the last 60 days. The meters are quiet."
                                    </p>
                                }
                                .into_view()
                            } else {
                                list.into_iter()
                                    .map(|a: Alert| {
                                        view! {
                                            <div class="alert" role="listitem">
                                                <div class="alert-meta">
                                                    <span class="alert-building">{a.label.clone()}</span>
                                                    <span class="alert-metric">
                                                        {metric_label(&a.metric).to_string()}
                                                    </span>
                                                <span class="alert-date">{a.date.clone()}</span>
                                                    <span class="alert-sigma">
                                                        {format!("{:.1} sigma", a.ratio_sigma)}
                                                    </span>
                                                </div>
                                                <p class="alert-reason">{a.reason}</p>
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            }
                        }}
                    </div>
                </div>
                <aside class="notes">
                    <SectionHead
                        num="04"
                        title="Field notes"
                        note="what the ledger says to do next"
                    />
                    <ul class="notes-list">
                        {move || {
                            suggestions
                                .get()
                                .into_iter()
                                .map(|s| {
                                    view! {
                                        <li class="note">
                                            <div class="note-head">
                                                <span class="note-building">{s.label.clone()}</span>
                                                <Tag kind={s.priority.clone()}>{s.priority.clone()}</Tag>
                                            </div>
                                            <p class="note-text">{s.text}</p>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </aside>
            </section>
        </div>
    }
}
