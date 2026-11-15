use crate::api::{find_building, Reading, WasteSplit, BUILDINGS};
use crate::charts::{LineChart, ScoreRing, WasteBars};
use crate::fmt::human_num;
use crate::state::Store;
use crate::ui::{SectionHead, Skeleton, reduced_motion};
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

pub fn next_frame(f: impl FnOnce() + 'static) {
    let slot: Rc<RefCell<Option<Box<dyn FnOnce() + 'static>>>> =
        Rc::new(RefCell::new(Some(Box::new(f))));
    let cb_slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let cb_slot2 = cb_slot.clone();
    let closure: Closure<dyn FnMut(f64)> = Closure::wrap(Box::new(move |_t: f64| {
        if let Some(func) = slot.borrow_mut().take() {
            func();
        }
    }) as Box<dyn FnMut(f64)>);
    *cb_slot.borrow_mut() = Some(closure);
    if let Some(w) = web_sys::window() {
        let held = cb_slot.borrow();
        if let Some(c) = held.as_ref() {
            let _ = w.request_animation_frame(c.as_ref().unchecked_ref());
        }
    }
}

fn measure_lb_rows(container: &web_sys::Element) -> Vec<(String, f64, web_sys::Element)> {
    let mut out = Vec::new();
    let children = container.children();
    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            if let Some(key) = child.get_attribute("data-lb-key") {
                let top = child.get_bounding_client_rect().top();
                out.push((key, top, child));
            }
        }
    }
    out
}

pub fn subtract_days(iso: &str, n: i64) -> String {
    let parts: Vec<i64> = iso.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 {
        return iso.to_string();
    }
    let civil = crate::demo::days_from_civil(parts[0], parts[1] as u32, parts[2] as u32);
    crate::demo::iso_date(civil - n)
}

#[component]
pub fn Buildings() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");
    let sort_mode = create_rw_signal("co2".to_string());

    let rows = create_memo(move |_| {
        let mode = sort_mode.get();
        let mut list = store.leaderboard.get().rows;
        match mode.as_str() {
            "kwh" => list.sort_by(|a, b| {
                b.kwh
                    .partial_cmp(&a.kwh)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.building.cmp(&b.building))
            }),
            "score" => list.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then_with(|| a.building.cmp(&b.building))
            }),
            "name" => list.sort_by(|a, b| a.label.cmp(&b.label)),
            _ => list.sort_by(|a, b| {
                a.co2_per_occupant
                    .partial_cmp(&b.co2_per_occupant)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.building.cmp(&b.building))
            }),
        }
        list
    });

    let container_ref = create_node_ref::<html::Div>();
    let prev_positions = create_rw_signal(Vec::<(String, f64)>::new());
    let prev_signature = create_rw_signal(String::new());

    create_effect(move |_| {
        let list = rows.get();
        let sig = list
            .iter()
            .map(|r| format!("{}|{:.2}|{}", r.building, r.co2_per_occupant, r.score))
            .collect::<Vec<_>>()
            .join(";");
        let prev_sig = prev_signature.get_untracked();
        prev_signature.set(sig);
        let sig_changed = sig != prev_sig;

        let Some(container) = container_ref.get() else { return };
        next_frame(move || {
            let measured = measure_lb_rows(&container);
            let positions: Vec<(String, f64)> =
                measured.iter().map(|(k, t, _)| (k.clone(), *t)).collect();
            if !sig_changed || reduced_motion() {
                prev_positions.set(positions);
                return;
            }
            let prev = prev_positions.get_untracked();
            let mut moved: Vec<web_sys::Element> = Vec::new();
            for (key, top, el) in &measured {
                if let Some((_, old_top)) = prev.iter().find(|(k, _)| k == key) {
                    let delta = old_top - top;
                    if delta.abs() > 0.5 {
                        moved.push(el.clone());
                        let _ = el
                            .set_attribute("style", &format!("transform: translateY({delta:.1}px)"));
                    }
                }
            }
            prev_positions.set(positions);
            if moved.is_empty() {
                return;
            }
            next_frame(move || {
                for el in &moved {
                    let _ = el.set_attribute("class", "lb-row flip");
                    let _ = el.set_attribute("style", "");
                }
                set_timeout(
                    move || {
                        for el in &moved {
                            let _ = el.set_attribute("class", "lb-row");
                        }
                    },
                    640,
                );
            });
        });
    });

    let selected = create_rw_signal("hostel-a".to_string());
    let detail_note = create_memo(move |_| {
        let id = selected.get();
        find_building(&id)
            .map(|b| format!("{} · {} occupants · {}", b.label, b.occupants, b.kind))
            .unwrap_or_default()
    });
    let detail_loading = create_rw_signal(false);
    create_effect(move |_| {
        let _ = selected.get();
        detail_loading.set(true);
        let delay = if reduced_motion() { 0 } else { 240 };
        set_timeout(move || detail_loading.set(false), delay);
    });

    let detail = create_memo(move |_| {
        let id = selected.get();
        let window_end = store.summary.get().window_end.clone();
        let readings = store.readings.get();
        let mut dates: Vec<String> = Vec::new();
        for r in readings.iter().filter(|r| r.building == id) {
            if !dates.contains(&r.date) {
                dates.push(r.date.clone());
            }
        }
        dates.sort();
        let mut days: Vec<(String, f64)> = Vec::with_capacity(dates.len());
        for d in &dates {
            let sum: f64 = readings
                .iter()
                .filter(|r| r.building == id && &r.date == d)
                .map(|r| r.kwh)
                .sum();
            days.push((d.clone(), sum));
        }
        let water_sum: f64 = readings
            .iter()
            .filter(|r| r.building == id)
            .map(|r| r.water_l)
            .sum();
        let water_days = dates.len().max(1) as f64;
        let water_avg = water_sum / water_days;
        let start = subtract_days(&window_end, 6);
        let split = WasteSplit {
            organic: readings
                .iter()
                .filter(|r| r.building == id && r.date >= start && r.date <= window_end)
                .map(|r| r.waste_kg.organic)
                .sum(),
            plastic: readings
                .iter()
                .filter(|r| r.building == id && r.date >= start && r.date <= window_end)
                .map(|r| r.waste_kg.plastic)
                .sum(),
            paper: readings
                .iter()
                .filter(|r| r.building == id && r.date >= start && r.date <= window_end)
                .map(|r| r.waste_kg.paper)
                .sum(),
            ewaste: readings
                .iter()
                .filter(|r| r.building == id && r.date >= start && r.date <= window_end)
                .map(|r| r.waste_kg.ewaste)
                .sum(),
        };
        let total = split.total();
        let row = store
            .leaderboard
            .get()
            .rows
            .into_iter()
            .find(|r| r.building == id);
        (days, split, total, row, water_avg, dates)
    });

    view! {
        <div class="buildings">
            <section class="section">
                <SectionHead
                    num="01"
                    title="Leaderboard"
                    note="CO2 per occupant over the trailing seven days · lower is better"
                />
                <div class="lb-sort" role="group" aria-label="Sort leaderboard">
                    <SortButton label="co2 / occupant" value="co2" mode=sort_mode/>
                    <SortButton label="electricity" value="kwh" mode=sort_mode/>
                    <SortButton label="score" value="score" mode=sort_mode/>
                    <SortButton label="name" value="name" mode=sort_mode/>
                </div>
                <div class="lb-head" aria-hidden="true">
                    <span>rank</span>
                    <span>building</span>
                    <span>people</span>
                    <span>electricity</span>
                    <span>co2 / person</span>
                    <span>score</span>
                </div>
                <div class="lb-rows" node_ref=container_ref role="table" aria-label="Buildings ranked by CO2 per occupant">
                    {move || {
                        rows
                            .get()
                            .into_iter()
                            .map(|r| {
                                let key = r.building.clone();
                                let rank = format!("{:02}", r.rank);
                                let label = r.label.clone();
                                let occ = r.occupants;
                                let kwh_txt = format!("{} kWh", human_num(r.kwh, 0));
                                let co2_txt = format!("{} kg", human_num(r.co2_per_occupant, 2));
                                let score = r.score;
                                view! {
                                    <div class="lb-row" data-lb-key={key}>
                                        <span class="lb-rank">{rank}</span>
                                        <span class="lb-name">{label}</span>
                                        <span class="lb-occ">{occ}</span>
                                        <span class="lb-kwh">{kwh_txt}</span>
                                        <span class="lb-co2">{co2_txt}</span>
                                        <span class="lb-score">
                                            <ScoreRing score=score size=52.0/>
                                        </span>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
                <p class="chart-note">
                    Scores put the best building at 100 and the heaviest emitter at 20.
                    The gap between hostels and teaching space is the story: residents
                    carry the AC load.
                </p>
            </section>

            <div class="rule rule-anim" aria-hidden="true"></div>

            <section class="section">
                <SectionHead
                    num="02"
                    title="Building detail"
                    note=detail_note
                />
                <div class="bselector" role="group" aria-label="Choose a building">
                    {move || {
                        BUILDINGS
                            .iter()
                            .map(|b| {
                                view! {
                                    <button
                                        type="button"
                                        class="btab"
                                        class:on=move || selected.get() == b.id
                                        on:click=move |_| selected.set(b.id.to_string())
                                    >
                                        {b.label.to_string()}
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                </div>
                {move || {
                    if detail_loading.get() {
                        view! { <Skeleton lines=5/> }.into_view()
                    } else {
                        let (days, split, total, row, water_avg, dates) = detail.get();
                        let per_occ = row.as_ref().map(|r| r.co2_per_occupant).unwrap_or(0.0);
                        let score_val = row.as_ref().map(|r| r.score).unwrap_or(0);
                        let kwh7 = row.as_ref().map(|r| r.kwh).unwrap_or(0.0);
                        let avg_kwh = if days.is_empty() {
                            0.0
                        } else {
                            days.iter().map(|p| p.1).sum::<f64>() / days.len() as f64
                        };
                        let chart_aria = format!(
                            "{} daily electricity over the last {} days, averaging {} kWh",
                            selected.get_untracked(),
                            dates.len(),
                            human_num(avg_kwh, 0)
                        );
                        view! {
                            <div class="bdetail">
                                <div class="bdetail-side">
                                    <ScoreRing score=score_val size=150.0 caption="score"/>
                                    <div class="bdetail-facts">
                                        <div class="fact">
                                            <span class="fact-label">electricity · 7d</span>
                                            <span class="fact-value">{human_num(kwh7, 0)} kWh</span>
                                        </div>
                                        <div class="fact">
                                            <span class="fact-label">co2 per person · 7d</span>
                                            <span class="fact-value">{human_num(per_occ, 2)} kg</span>
                                        </div>
                                        <div class="fact">
                                            <span class="fact-label">water · daily mean</span>
                                            <span class="fact-value">{human_num(water_avg, 0)} L</span>
                                        </div>
                                    </div>
                                </div>
                                <div class="bdetail-main">
                                    <p class="panel-title">electricity · trailing 60 days</p>
                                    <LineChart points=days unit=" kWh" aria={chart_aria}/>
                                    <p class="panel-title">
                                        "waste · trailing 7 days · " {human_num(total, 1)} " kg total"
                                    </p>
                                    <WasteBars split=split total=total/>
                                </div>
                            </div>
                        }
                        .into_view()
                    }
                }}
            </section>
        </div>
    }
}

#[component]
fn SortButton(
    label: &'static str,
    value: &'static str,
    mode: RwSignal<String>,
) -> impl IntoView {
    let on = create_memo(move |_| mode.get() == value);
    view! {
        <button
            type="button"
            class="lb-sort-btn"
            class:on=on
            on:click=move |_| mode.set(value.to_string())
        >
            {label}
        </button>
    }
}
