use crate::api::WasteSplit;
use crate::fmt::human_num;
use crate::ui::{animate, reduced_motion};
use leptos::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

const VIEW_W: f64 = 760.0;
const VIEW_H: f64 = 300.0;
const PAD_X: f64 = 14.0;
const PAD_TOP: f64 = 26.0;
const PAD_BOTTOM: f64 = 40.0;

#[component]
pub fn LineChart(
    #[prop(into)] points: MaybeSignal<Vec<(String, f64)>>,
    #[prop(optional)] anomalies: Option<Memo<Vec<usize>>>,
    #[prop(default = "")] unit: &'static str,
    #[prop(into)] aria: MaybeSignal<String>,
) -> impl IntoView {
    let anomalies = anomalies.unwrap_or_else(|| Memo::new(move |_| Vec::new()));
    let vals = create_rw_signal(Vec::<f64>::new());
    let labels = create_rw_signal(Vec::<String>::new());

    create_effect(move |_| {
        let pts = points.get();
        labels.set(pts.iter().map(|p| p.0.clone()).collect());
        if pts.is_empty() {
            vals.set(Vec::new());
            return;
        }
        let new_vals: Vec<f64> = pts.iter().map(|p| p.1).collect();
        if reduced_motion() {
            vals.set(new_vals);
            return;
        }
        let old = vals.get_untracked();
        let old_len = old.len();
        let from: Rc<Vec<f64>> = Rc::new(
            (0..new_vals.len())
                .map(|i| old.get(i.min(old_len.saturating_sub(1))).copied().unwrap_or(0.0))
                .collect(),
        );
        let target: Rc<Vec<f64>> = Rc::new(new_vals);
        animate(700.0, move |p| {
            let cur: Vec<f64> = target
                .iter()
                .zip(from.iter())
                .map(|(t, f)| f + (t - f) * p)
                .collect();
            vals.set_untracked(cur);
        });
    });

    let hovered = create_rw_signal::<Option<usize>>(None);
    let pointer = create_rw_signal((0.0f64, 0.0f64));

    let on_move = move |e: web_sys::MouseEvent| {
        let Some(target) = e.current_target() else { return };
        let el: web_sys::Element = target.unchecked_into();
        let rect = el.get_bounding_client_rect();
        let rel_x = e.client_x() as f64 - rect.left();
        let rel_y = e.client_y() as f64 - rect.top();
        pointer.set((rel_x, rel_y));
        let v = vals.get_untracked();
        if v.len() < 2 {
            return;
        }
        let span_x = VIEW_W - PAD_X * 2.0;
        let vx = rel_x * (VIEW_W / rect.width().max(1.0));
        let step = span_x / (v.len() - 1) as f64;
        let idx = ((vx - PAD_X) / step).round().clamp(0.0, (v.len() - 1) as f64) as usize;
        hovered.set(Some(idx));
    };

    let path_d = create_memo(move |_| -> String {
        let v = vals.get();
        if v.len() < 2 {
            return String::new();
        }
        let span_x = VIEW_W - PAD_X * 2.0;
        let min = v.iter().cloned().fold(f64::MAX, f64::min);
        let max = v.iter().cloned().fold(f64::MIN, f64::max);
        let (lo, hi) = if (max - min).abs() < 1e-9 {
            (min - 1.0, max + 1.0)
        } else {
            (min, max)
        };
        let span_y = VIEW_H - PAD_TOP - PAD_BOTTOM;
        let n = v.len();
        let mut d = String::new();
        for (i, val) in v.iter().enumerate() {
            let x = PAD_X + i as f64 / (n - 1) as f64 * span_x;
            let y = PAD_TOP + (1.0 - (val - lo) / (hi - lo)) * span_y;
            if i == 0 {
                d.push_str(&format!("M {x:.2} {y:.2}"));
            } else {
                d.push_str(&format!(" L {x:.2} {y:.2}"));
            }
        }
        d
    });

    let marks = create_memo(move |_| {
        let v = vals.get();
        let ls = labels.get();
        if v.len() < 2 {
            return Vec::<View>::new().collect_view();
        }
        let min = v.iter().cloned().fold(f64::MAX, f64::min);
        let max = v.iter().cloned().fold(f64::MIN, f64::max);
        let (lo, hi) = if (max - min).abs() < 1e-9 {
            (min - 1.0, max + 1.0)
        } else {
            (min, max)
        };
        let span_x = VIEW_W - PAD_X * 2.0;
        let span_y = VIEW_H - PAD_TOP - PAD_BOTTOM;
        let n = v.len();
        let anom = anomalies.get();
        let mut nodes: Vec<View> = Vec::new();
        for (i, val) in v.iter().enumerate() {
            let x = PAD_X + i as f64 / (n - 1) as f64 * span_x;
            let y = PAD_TOP + (1.0 - (val - lo) / (hi - lo)) * span_y;
            if anom.contains(&i) {
                nodes.push(view! {
                    <g class="anom">
                        <circle class="anom-dot" cx=x cy=y r="3.2"/>
                        <circle class="anom-ping" cx=x cy=y r="5"/>
                        <circle class="anom-ping anom-ping-2" cx=x cy=y r="5"/>
                    </g>
                }
                .into_view());
            }
            let show_label = n <= 8 || i == 0 || i == n - 1 || i % (n / 5).max(1) == 0;
            if show_label && i < ls.len() {
                let label = ls[i].clone();
                nodes.push(view! {
                    <text class="axis-label" x=x y={VIEW_H - PAD_BOTTOM + 18.0} text-anchor="middle">
                        {label}
                    </text>
                }
                .into_view());
            }
        }
        nodes.collect_view()
    });

    let hover_group = create_memo(move |_| -> Option<(f64, f64)> {
        let idx = hovered.get()?;
        let v = vals.get();
        if v.len() < 2 {
            return None;
        }
        let min = v.iter().cloned().fold(f64::MAX, f64::min);
        let max = v.iter().cloned().fold(f64::MIN, f64::max);
        let (lo, hi) = if (max - min).abs() < 1e-9 {
            (min - 1.0, max + 1.0)
        } else {
            (min, max)
        };
        let span_x = VIEW_W - PAD_X * 2.0;
        let span_y = VIEW_H - PAD_TOP - PAD_BOTTOM;
        let x = PAD_X + idx as f64 / (v.len() - 1) as f64 * span_x;
        let y = PAD_TOP + (1.0 - (v[idx] - lo) / (hi - lo)) * span_y;
        Some((x, y))
    });

    let tip = create_memo(move |_| -> Option<(String, f64, bool)> {
        let idx = hovered.get()?;
        let v = vals.get();
        let ls = labels.get();
        let label = ls.get(idx)?.clone();
        let value = *v.get(idx)?;
        let is_anom = anomalies.with(|a| a.contains(&idx));
        Some((label, value, is_anom))
    });

    view! {
        <div class="chart-wrap" on:mousemove=on_move on:mouseleave=move |_| hovered.set(None)>
            <svg
                class="chart-svg"
                viewBox=format!("0 0 {VIEW_W} {VIEW_H}")
                role="img"
                aria-label=move || aria.get()
            >
                <line class="grid" x1=PAD_X y1=PAD_TOP x2={VIEW_W - PAD_X} y2=PAD_TOP/>
                <line
                    class="grid"
                    x1=PAD_X
                    y1={PAD_TOP + (VIEW_H - PAD_TOP - PAD_BOTTOM) / 2.0}
                    x2={VIEW_W - PAD_X}
                    y2={PAD_TOP + (VIEW_H - PAD_TOP - PAD_BOTTOM) / 2.0}
                />
                <line class="grid" x1=PAD_X y1={VIEW_H - PAD_BOTTOM} x2={VIEW_W - PAD_X} y2={VIEW_H - PAD_BOTTOM}/>
                {move || marks.get()}
                <path class="line draw-line" d={move || path_d.get()} pathLength="1000"/>
                {move || {
                    hover_group.get().map(|(x, y)| {
                        view! {
                            <g class="crosshair">
                                <line x1=x y1=PAD_TOP x2=x y2={VIEW_H - PAD_BOTTOM}/>
                                <circle class="crosshair-dot" cx=x cy=y r="3.5"/>
                            </g>
                        }
                    })
                }}
            </svg>
            {move || {
                tip.get().map(|(label, value, is_anom)| {
                    let (px, py) = pointer.get();
                    let (tx, ty) = if px > 170.0 { (px - 14.0, py - 14.0) } else { (px + 16.0, py + 16.0) };
                    let ty = if ty > 130.0 { ty - 82.0 } else { ty };
                    let unit_s = if unit.is_empty() { String::new() } else { unit.to_string() };
                    view! {
                        <div
                            class="chart-tip"
                            class:tip-anom=is_anom
                            style=format!("transform: translate({tx:.1}px, {ty:.1}px)")
                        >
                            <div class="tip-label">{label}</div>
                            <div class="tip-value">{human_num(value, 1)}{unit_s}</div>
                        </div>
                    }
                })
            }}
        </div>
    }
}

#[component]
pub fn WasteBars(
    #[prop(into)] split: MaybeSignal<WasteSplit>,
    #[prop(into)] total: MaybeSignal<f64>,
) -> impl IntoView {
    let rows = create_memo(move |_| {
        let s = split.get();
        let t = total.get().max(1.0);
        let raw: [(&str, &str, f64, &str); 4] = [
            ("organic", "Organic", s.organic, "solid"),
            ("plastic", "Plastic", s.plastic, "hatch"),
            ("paper", "Paper", s.paper, "dotted"),
            ("ewaste", "E-waste", s.ewaste, "outline"),
        ];
        raw
            .into_iter()
            .map(|(key, label, value, style)| {
                let pct = (value / t * 100.0).clamp(0.0, 100.0);
                (key.to_string(), label, value, pct.to_string(), style)
            })
            .collect::<Vec<_>>()
    });
    view! {
        <div class="wb" role="img" aria-label="Waste categories by weight">
            {move || {
                rows.get()
                    .into_iter()
                    .enumerate()
                    .map(|(i, (key, label, value, pct, style))| {
                        view! {
                            <div class="wb-row" data-cat={key}>
                                <span class="wb-name">{label}</span>
                                <span class="wb-track">
                                    <span
                                        class="wb-bar"
                                        data-fill={style}
                                        style=format!("width: {pct}%; --i: {i}")
                                    ></span>
                                </span>
                                <span class="wb-value">{human_num(value, 1)} kg</span>
                                <span class="wb-pct">{pct}%</span>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

#[component]
pub fn ScoreRing(
    #[prop(into)] score: MaybeSignal<u32>,
    #[prop(default = 128.0)] size: f64,
    #[prop(default = "")] caption: &'static str,
) -> impl IntoView {
    let r = (size - 14.0) / 2.0;
    let c = 2.0 * std::f64::consts::PI * r;
    let shown = create_rw_signal(if reduced_motion() { score.get_untracked() as f64 } else { 0.0 });
    create_effect(move |_| {
        let target = score.get() as f64;
        if reduced_motion() {
            shown.set(target);
            return;
        }
        let from = shown.get_untracked();
        animate(1100.0, move |p| {
            shown.set_untracked(from + (target - from) * p);
        });
    });
    let offset = create_memo(move |_| c * (1.0 - (shown.get() / 100.0).clamp(0.0, 1.0)));
    let aria = create_memo(move |_| format!("sustainability score {} of 100", score.get()));
    view! {
        <div class="ring" style=format!("width: {size:.0}px; height: {size:.0}px")>
            <svg viewBox=format!("0 0 {size:.0} {size:.0}") role="img" aria-label=aria>
                <circle class="ring-track" cx=size/2.0 cy=size/2.0 r=r/>
                <circle
                    class="ring-fill"
                    cx=size/2.0
                    cy=size/2.0
                    r=r
                    stroke-dasharray=format!("{c:.2}")
                    stroke-dashoffset=move || format!("{:.2}", offset.get())
                    transform=format!("rotate(-90 {:.1} {:.1})", size / 2.0, size / 2.0)
                />
            </svg>
            <div class="ring-center">
                <span class="ring-value">{move || format!("{:.0}", shown.get())}</span>
                {if caption.is_empty() {
                    None
                } else {
                    Some(view! { <span class="ring-caption">{caption}</span> })
                }}
            </div>
        </div>
    }
}
