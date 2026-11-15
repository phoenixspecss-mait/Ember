use crate::fmt;
use crate::state::{Store, Toast};
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

pub fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

pub fn animate(duration_ms: f64, mut on_frame: impl FnMut(f64) + 'static) {
    if reduced_motion() {
        on_frame(1.0);
        return;
    }
    let start = js_sys::Date::now();
    let slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let slot2 = slot.clone();
    let closure: Closure<dyn FnMut(f64)> = Closure::wrap(Box::new(move |_t: f64| {
        let elapsed = js_sys::Date::now() - start;
        let p = if duration_ms <= 0.0 { 1.0 } else { (elapsed / duration_ms).min(1.0) };
        let eased = 1.0 - (1.0 - p) * (1.0 - p) * (1.0 - p);
        on_frame(eased);
        if p < 1.0 {
            let held = slot2.borrow();
            if let (Some(c), Some(w)) = (held.as_ref(), web_sys::window()) {
                let _ = w.request_animation_frame(c.as_ref().unchecked_ref());
            }
        }
    }) as Box<dyn FnMut(f64)>);
    *slot.borrow_mut() = Some(closure);
    if let Some(w) = web_sys::window() {
        let held = slot.borrow();
        if let Some(c) = held.as_ref() {
            let _ = w.request_animation_frame(c.as_ref().unchecked_ref());
        }
    }
}

#[component]
pub fn CountUp(
    #[prop(into)] value: Signal<f64>,
    #[prop(default = 0)] decimals: usize,
    #[prop(default = "")] prefix: &'static str,
    #[prop(default = "")] suffix: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let shown = create_rw_signal(if reduced_motion() { value.get_untracked() } else { 0.0 });
    let gen = create_rw_signal(0u64);
    create_effect(move |_| {
        let target = value.get();
        gen.update(|g| *g += 1);
        let mine = gen.get_untracked();
        if reduced_motion() {
            shown.set(target);
            return;
        }
        let from = shown.get_untracked();
        let duration = if from == 0.0 { 950.0 } else { 700.0 };
        animate(duration, move |p| {
            if gen.get_untracked() == mine {
                shown.set_untracked(from + (target - from) * p);
            }
        });
    });
    let text = create_memo(move |_| {
        format!("{prefix}{}{suffix}", fmt::human_num(shown.get(), decimals))
    });
    view! { <span class=format!("num {class}")>{move || text.get()}</span> }
}

#[component]
pub fn Delta(#[prop(into)] value: Signal<f64>, #[prop(default = 1)] decimals: usize) -> impl IntoView {
    let text = create_memo(move |_| {
        let v = value.get();
        let body = fmt::human_num(v.abs(), decimals);
        if v >= 0.0 {
            format!("+{body}")
        } else {
            format!("-{body}")
        }
    });
    view! { <span class="num delta">{move || text.get()}</span> }
}

#[component]
pub fn Tag(#[prop(into)] kind: String, children: Children) -> impl IntoView {
    view! { <span class=format!("tag tag-{kind}")>{children()}</span> }
}

#[component]
pub fn SectionHead(
    num: &'static str,
    title: &'static str,
    #[prop(optional, into)] note: MaybeSignal<String>,
) -> impl IntoView {
    view! {
        <header class="shead">
            <div class="shead-rule" aria-hidden="true"></div>
            <span class="shead-num" aria-hidden="true">{num}</span>
            <h2 class="shead-title">
                <span class="mask"><span class="mask-inner">{title}</span></span>
            </h2>
            {move || {
                let n = note.get();
                if n.is_empty() {
                    None
                } else {
                    Some(view! { <p class="shead-note">{n}</p> })
                }
            }}
        </header>
    }
}

#[component]
pub fn Skeleton(#[prop(default = 4)] lines: u32) -> impl IntoView {
    view! {
        <div class="skeleton" aria-hidden="true">
            {(0..lines).map(|_| view! { <div class="sk-line"></div> }).collect_view()}
        </div>
    }
}

pub fn metric_label(metric: &str) -> &'static str {
    match metric {
        "kwh" => "electricity",
        "water_l" => "water",
        _ => "waste",
    }
}

pub fn metric_unit(metric: &str) -> &'static str {
    match metric {
        "kwh" => "kWh",
        "water_l" => "L",
        _ => "kg",
    }
}

#[component]
pub fn Toasts() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");
    view! {
        <div class="toasts" aria-live="polite" aria-label="New anomalies">
            {move || {
                store
                    .toasts
                    .get()
                    .into_iter()
                    .map(|t: Toast| {
                        view! {
                            <div class="toast">
                                <div class="toast-head">
                                    <span class="toast-metric">{metric_label(&t.metric).to_string()}</span>
                                    <span class="toast-date">{t.label}{" · "}{t.date}</span>
                                </div>
                                <p class="toast-body">{t.reason}</p>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}
