use crate::api::{api_base, BUILDINGS};
use crate::charts::ScoreRing;
use crate::state::Store;
use crate::ui::{CountUp, SectionHead};
use leptos::*;
use leptos_router::A;

#[component]
pub fn Landing() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");
    let live = api_base().is_some();

    let kwh = create_memo(move |_| store.summary.get().kwh);
    let co2 = create_memo(move |_| store.summary.get().co2_kg);
    let score = create_memo(move |_| store.summary.get().campus_score);
    let alerts = create_memo(move |_| store.alerts.get().count as f64);
    let window_end = create_memo(move |_| store.summary.get().window_end);
    let open_today = create_memo(move |_| {
        if api_base().is_none() {
            return 0usize;
        }
        let today = crate::demo::iso_date(crate::demo::today_civil());
        let readings = store.readings.get();
        BUILDINGS
            .iter()
            .filter(|b| !readings.iter().any(|r| r.building == b.id && r.date == today))
            .count()
    });

    view! {
        <div class="landing">
            <section class="land-hero">
                <span class="giant-num" aria-hidden="true">00</span>
                <div class="land-main">
                    <p class="kicker rise">
                        <span class="mask">
                            <span class="mask-inner">"Campus energy & waste ledger"</span>
                        </span>
                    </p>
                    <h1 class="land-title">
                        <span class="mask"><span class="mask-inner">"The campus,"</span></span>
                        <span class="mask">
                            <span class="mask-inner land-accent" style="--d: .14s">"metered."</span>
                        </span>
                    </h1>
                    <p class="land-lede">
                        "Six buildings report five numbers a day: electricity, water, and three streams of waste. Ember turns the meter history into carbon, anomalies, and the next right action."
                    </p>
                    <div class="land-cta">
                        <A href="/overview" class="btn btn-primary">"Get started"</A>
                        {move || {
                            if live {
                                Some(view! { <A href="/log" class="btn">"File a reading"</A> })
                            } else {
                                None
                            }
                        }}
                    </div>
                    {move || {
                        let n = open_today.get();
                        if n == 0 {
                            None
                        } else {
                            Some(view! {
                                <p class="land-open">
                                    <span class="open-dot" aria-hidden="true"></span>
                                    {format!("{n} of {} buildings still to read today", BUILDINGS.len())}
                                    <A href="/log" class="land-open-link">"file yours →"</A>
                                </p>
                            })
                        }
                    }}
                </div>
                <aside class="land-plate">
                    <div class="plate-top">
                        <span>"ledger · seven complete days"</span>
                        <span class="plate-date">{move || window_end.get()}</span>
                    </div>
                    <div class="plate-ring">
                        <ScoreRing score=score size=188.0 caption="campus score"/>
                    </div>
                    <div class="plate-rows">
                        <div class="plate-row">
                            <span class="plate-key">"electricity"</span>
                            <span class="plate-val">
                                <CountUp value=kwh decimals=0/>" kWh"
                            </span>
                        </div>
                        <div class="plate-row">
                            <span class="plate-key">"carbon"</span>
                            <span class="plate-val">
                                <CountUp value=co2 decimals=0/>" kg"
                            </span>
                        </div>
                        <div class="plate-row">
                            <span class="plate-key">"anomalies on file"</span>
                            <span class="plate-val">
                                <CountUp value=alerts decimals=0/>
                            </span>
                        </div>
                    </div>
                    <p class="plate-foot">
                        "0.82 kg CO₂ per kWh · six buildings · one shared ledger"
                    </p>
                </aside>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead num="01" title="How the ledger works" note="read · file · act"/>
                <div class="land-steps">
                    <article class="land-step">
                        <span class="land-step-num">"01"</span>
                        <h3 class="land-step-title">"Read the meters"</h3>
                        <p class="land-step-text">
                            "One row per building per day: kWh, litres, and waste split across organic, plastic, paper and e-waste."
                        </p>
                    </article>
                    <article class="land-step">
                        <span class="land-step-num">"02"</span>
                        <h3 class="land-step-title">"Let it do the math"</h3>
                        <p class="land-step-text">
                            "Carbon totals, a rolling seven-day anomaly test, per-occupant rankings and a 0–100 campus score, computed server-side."
                        </p>
                    </article>
                    <article class="land-step">
                        <span class="land-step-num">"03"</span>
                        <h3 class="land-step-title">"Act on the field notes"</h3>
                        <p class="land-step-text">
                            "Rule-based suggestions ranked by priority: leak walks, schedule fixes, bin-split retraining, certified e-waste pickups."
                        </p>
                    </article>
                </div>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead
                    num="02"
                    title="What every number means"
                    note="no black boxes · the formulas are public"
                />
                <dl class="land-formulas">
                    <div class="land-formula">
                        <dt>"carbon"</dt>
                        <dd class="formula">"kg CO₂ = 0.82 × kWh"</dd>
                        <dd class="formula-note">"the grid emission factor"</dd>
                    </div>
                    <div class="land-formula">
                        <dt>"anomaly"</dt>
                        <dd class="formula">"v > mean + 2σ  and  v > 1.40 × mean"</dd>
                        <dd class="formula-note">"rolling seven-day window, per building and meter"</dd>
                    </div>
                    <div class="land-formula">
                        <dt>"subscore"</dt>
                        <dd class="formula">"100 − 220 × (ratio − 0.88)"</dd>
                        <dd class="formula-note">
                            "energy 0.45 · water 0.30 · waste 0.25, per occupant against the building's own norm"
                        </dd>
                    </div>
                    <div class="land-formula">
                        <dt>"rank"</dt>
                        <dd class="formula">"kg CO₂ per occupant"</dd>
                        <dd class="formula-note">"best building 100 · heaviest emitter 20"</dd>
                    </div>
                </dl>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead num="03" title="On the meter" note="six buildings · one ledger"/>
                <div class="land-roll">
                    {move || {
                        let today = crate::demo::iso_date(crate::demo::today_civil());
                        let readings = store.readings.get();
                        BUILDINGS
                            .iter()
                            .map(|b| {
                                let filed = readings.iter().any(|r| r.building == b.id && r.date == today);
                                let state_class = if !live || filed {
                                    "land-row-state filed"
                                } else {
                                    "land-row-state open"
                                };
                                let state_text = if !live {
                                    "demo · on file"
                                } else if filed {
                                    "today · filed"
                                } else {
                                    "today · open"
                                };
                                view! {
                                    <div class="land-row">
                                        <span class="land-row-name">{b.label.to_string()}</span>
                                        <span class="land-row-kind">{b.kind.to_string()}</span>
                                        <span class="land-row-occ">
                                            {format!("{} people", b.occupants)}
                                        </span>
                                        <span class=state_class>{state_text}</span>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
                <div class="land-final">
                    {move || {
                        if !live {
                            view! {
                                <p class="land-final-text">
                                    "This build runs on the demo ledger. The dashboard, formulas, and field notes are fully live; filing readings needs the API."
                                </p>
                                <div class="land-final-cta">
                                    <A href="/overview" class="btn btn-primary">
                                        "Open the dashboard"
                                    </A>
                                </div>
                            }
                                .into_view()
                        } else {
                            let n = open_today.get();
                            if n > 0 {
                                view! {
                                    <p class="land-final-text">
                                        {format!(
                                            "{n} of {} meters are still open today. The day joins the week once all six are in.",
                                            BUILDINGS.len(),
                                        )}
                                    </p>
                                    <div class="land-final-cta">
                                        <A href="/log" class="btn btn-primary">
                                            "File today's reading"
                                        </A>
                                        <A href="/overview" class="btn">
                                            "Open the dashboard"
                                        </A>
                                    </div>
                                }
                                    .into_view()
                            } else {
                                view! {
                                    <p class="land-final-text">
                                        "Every meter is on file for today. The day is closed and the week updated."
                                    </p>
                                    <div class="land-final-cta">
                                        <A href="/overview" class="btn btn-primary">
                                            "Open the dashboard"
                                        </A>
                                        <A href="/log" class="btn">
                                            "File a correction"
                                        </A>
                                    </div>
                                }
                                    .into_view()
                            }
                        }
                    }}
                </div>
            </section>
        </div>
    }
}
