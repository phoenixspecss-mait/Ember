use crate::ui::reduced_motion;
use leptos::*;

const LEAVES: [(f64, f64, f64); 27] = [
    (118.0, 96.0, -28.0),
    (128.0, 82.0, 12.0),
    (141.0, 70.0, -8.0),
    (155.0, 62.0, 24.0),
    (170.0, 58.0, -18.0),
    (186.0, 63.0, 8.0),
    (200.0, 72.0, -32.0),
    (211.0, 86.0, 18.0),
    (218.0, 102.0, -10.0),
    (213.0, 118.0, 30.0),
    (198.0, 128.0, -24.0),
    (182.0, 134.0, 6.0),
    (166.0, 137.0, -14.0),
    (150.0, 135.0, 26.0),
    (135.0, 128.0, -6.0),
    (124.0, 114.0, 16.0),
    (147.0, 88.0, -20.0),
    (163.0, 84.0, 14.0),
    (179.0, 92.0, -28.0),
    (192.0, 104.0, 10.0),
    (178.0, 110.0, -12.0),
    (160.0, 108.0, 22.0),
    (143.0, 104.0, -34.0),
    (132.0, 102.0, 8.0),
    (204.0, 96.0, -16.0),
    (139.0, 116.0, 18.0),
    (173.0, 120.0, -8.0),
];

const BRANCHES: [&str; 7] = [
    "M 159.5 196 C 150 182 138 170 124 158 C 118 152 118 150 116 143",
    "M 160.5 188 C 172 174 186 164 196 152 C 202 145 206 140 208 133",
    "M 159.5 166 C 152 150 147 136 143 121 C 141 113 141 108 142 100",
    "M 160.5 160 C 170 146 178 133 186 122 C 190 115 193 110 196 103",
    "M 160 143 C 158 128 158 114 158 100 C 158 92 159 86 160 80",
    "M 159.5 132 C 150 124 141 118 132 112 C 127 109 124 106 121 102",
    "M 160.5 136 C 170 127 180 120 189 114 C 194 111 198 108 201 105",
];

#[component]
pub fn CampusTree(#[prop(into)] score: Signal<u32>) -> impl IntoView {
    let total = LEAVES.len();
    let leaf_count = create_memo(move |_| {
        let s = score.get().clamp(0, 100) as usize;
        let count = (s * total + 50) / 100;
        count.min(total)
    });

    let prev = create_rw_signal(score.get_untracked());
    let shed = create_rw_signal(Vec::<(f64, f64, u64)>::new());
    let shed_id = create_rw_signal(0u64);

    create_effect(move |_| {
        let s = score.get();
        let old = prev.get_untracked();
        prev.set(s);
        if reduced_motion() || s >= old {
            return;
        }
        let count_now = leaf_count.get_untracked();
        let lost = (old - s) as usize;
        let lost = lost.min(6).max(1);
        let mut drops: Vec<(f64, f64, u64)> = Vec::new();
        for k in 0..lost {
            let idx = (count_now + k * 3 + 1) % total;
            let (x, y, _) = LEAVES[idx];
            shed_id.update(|id| {
                *id += 1;
                drops.push((x + ((k % 3) as f64 - 1.0) * 7.0, y, *id));
            });
        }
        shed.update(|list| list.append(&mut drops));
        for (x, y, id) in shed.get_untracked() {
            set_timeout(
                move || {
                    shed.update(|list| list.retain(|(lx, ly, lid)| !(lx == x && ly == y && lid == id)));
                },
                2600,
            );
        }
    });

    let leaf_delay = |i: usize| -> String {
        let base = if reduced_motion() { 0.0 } else { 1.9 };
        format!("{:.2}s", base + (i % 6) as f64 * 0.09 + (i / 6) as f64 * 0.11)
    };

    view! {
        <div class="tree">
            <svg viewBox="0 0 320 300" role="img" aria-label="A hand-drawn tree that grows as the campus score improves">
                <line class="tree-ground" x1="52" y1="264" x2="268" y2="264"/>
                <path class="tree-trunk draw-line" pathLength="1000" d="M 160 263 C 156 246 162 232 159 214 C 157 202 158 192 159 182 C 160 172 159 164 160 158"/>
                <path class="tree-root draw-line" pathLength="1000" d="M 160 263 C 152 260 144 258 136 256"/>
                <path class="tree-root draw-line" pathLength="1000" d="M 160 263 C 169 260 177 258 184 257"/>
                {BRANCHES
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        view! {
                            <path class="tree-branch draw-line" pathLength="1000" style=format!("--d: {:.2}s", 0.55 + i as f64 * 0.16) d={*d}/>
                        }
                    })
                    .collect_view()}
                {move || {
                    LEAVES
                        .iter()
                        .enumerate()
                        .map(|(i, (x, y, rot))| {
                            let visible = i < leaf_count.get();
                            let style = format!("--d: {}", leaf_delay(i));
                            view! {
                                <g transform=format!("translate({x} {y}) rotate({rot})")>
                                    <path
                                        class="leaf"
                                        class:leaf-hidden=!visible
                                        style=style
                                        d="M0 0 C 3.6 -5.2 3.6 -12.5 0 -16.5 C -3.6 -12.5 -3.6 -5.2 0 0 Z"
                                    />
                                </g>
                            }
                        })
                        .collect_view()
                }}
                {move || {
                    shed.get()
                        .into_iter()
                        .map(|(x, y, id)| {
                            let style = format!("--dx: {:.0}px; --dy: {:.0}px", 14.0 + (id % 4) as f64 * 9.0, 150.0 + (id % 5) as f64 * 12.0);
                            view! {
                                <g transform=format!("translate({x} {y})")>
                                    <path
                                        class="leaf leaf-falling"
                                        style=style
                                        d="M0 0 C 3.6 -5.2 3.6 -12.5 0 -16.5 C -3.6 -12.5 -3.6 -5.2 0 0 Z"
                                    />
                                </g>
                            }
                        })
                        .collect_view()
                }}
            </svg>
            <p class="tree-caption">canopy tracks the campus score</p>
        </div>
    }
}
