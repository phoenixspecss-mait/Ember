use crate::api::*;
use crate::fmt::{clamp, human_num, int_text};

pub fn today_civil() -> i64 {
    (js_sys::Date::now() / 86_400_000.0) as i64
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let m = m as i64;
    let yy = y - if m <= 2 { 1 } else { 0 };
    let era = if yy >= 0 { yy } else { yy - 399 } / 400;
    let yoe = yy - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn iso_date(civil: i64) -> String {
    let z = civil + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let fy = y + if m <= 2 { 1 } else { 0 };
    format!("{fy:04}-{m:02}-{d:02}")
}

pub fn now_utc_iso() -> String {
    let secs = (js_sys::Date::now() / 1000.0) as i64;
    let day_secs = ((secs % 86400) + 86400) % 86400;
    let h = day_secs / 3600;
    let m = (day_secs % 3600) / 60;
    let s = day_secs % 60;
    let today = today_civil();
    format!("{}T{h:02}:{m:02}:{s:02}Z", iso_date(today))
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn f01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        let u1 = self.f01().max(1e-9);
        let u2 = self.f01();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

fn seed_round(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn seed_readings(today: i64) -> Vec<Reading> {
    let mut rng = Rng::new(2026_1104);
    let mut out = Vec::with_capacity(BUILDINGS.len() * 60);
    for d in 0..60i64 {
        let civil = today - 59 + d;
        let date = iso_date(civil);
        let dow = ((civil + 4) % 7 + 7) % 7;
        let weekend = dow == 0 || dow == 6;
        for b in BUILDINGS {
            let mut kwh = 0.0;
            let mut water = 0.0;
            let mut waste = 0.0;
            let mut split = WasteSplit { organic: 0.0, plastic: 0.0, paper: 0.0, ewaste: 0.0 };
            if b.kind == "hostel" {
                static KWH_BIAS: [f64; 4] = [1.00, 1.16, 1.02, 0.94];
                let idx = (b.id.as_bytes()[b.id.len() - 1] - b'a') as usize;
                let hot = rng.f01() < 0.30;
                let laundry = d % 7 == 2 || d % 7 == 3;
                kwh = (300.0 + b.occupants as f64 * 0.55)
                    * KWH_BIAS[idx]
                    * if weekend { 1.08 } else { 1.0 }
                    * if hot { 1.17 } else { 1.0 };
                water = b.occupants as f64 * 108.0 * if laundry { 1.25 } else { 1.0 };
                waste = b.occupants as f64 * 0.36;
                kwh *= 1.0 + 0.05 * rng.gauss();
                water *= 1.0 + 0.05 * rng.gauss();
                waste *= 1.0 + 0.09 * rng.gauss();
                split.organic = waste * 0.55;
                split.plastic = waste * 0.20;
                split.paper = waste * 0.235;
                split.ewaste = waste * 0.004;
            } else if b.kind == "library" {
                kwh = if weekend { if dow == 0 { 90.0 } else { 170.0 } } else { 300.0 };
                water = if weekend { if dow == 0 { 1800.0 } else { 2600.0 } } else { 6200.0 };
                waste = if weekend { if dow == 0 { 7.0 } else { 10.0 } } else { 18.0 };
                kwh *= 1.0 + 0.06 * rng.gauss();
                water *= 1.0 + 0.08 * rng.gauss();
                waste *= 1.0 + 0.10 * rng.gauss();
                split.organic = waste * 0.38;
                split.plastic = waste * 0.24;
                split.paper = waste * 0.36;
                split.ewaste = waste * 0.008;
            } else {
                kwh = if weekend { if dow == 0 { 260.0 } else { 300.0 } } else { 760.0 };
                water = if weekend { 10500.0 } else { 24000.0 };
                waste = if weekend { if dow == 0 { 55.0 } else { 60.0 } } else { 125.0 };
                kwh *= 1.0 + 0.06 * rng.gauss();
                water *= 1.0 + 0.07 * rng.gauss();
                waste *= 1.0 + 0.10 * rng.gauss();
                split.organic = waste * 0.42;
                split.plastic = waste * 0.26;
                split.paper = waste * 0.30;
                split.ewaste = waste * 0.006;
            }
            if rng.f01() < 0.02 {
                split.ewaste += 1.6;
            }
            if b.id == "main-block" && d == 33 {
                kwh *= 1.72;
            }
            if b.id == "hostel-c" && (d == 41 || d == 42) {
                water *= 2.45;
            }
            if b.id == "library" && d == 22 {
                split.organic *= 1.95;
                split.plastic *= 1.95;
                split.paper *= 1.95;
                split.ewaste *= 1.95;
                waste *= 1.95;
            }
            if b.id == "hostel-a" && (d == 12 || d == 13) {
                kwh *= 1.55;
            }
            if b.id == "hostel-d" && d == 55 {
                water *= 1.85;
            }
            if b.id == "main-block" && d == 57 {
                split.plastic *= 3.2;
                waste = split.total();
            }
            out.push(Reading {
                building: b.id.to_string(),
                date,
                kwh: seed_round(kwh.max(0.0)),
                water_l: seed_round(water.max(0.0)),
                waste_kg: WasteSplit {
                    organic: seed_round(split.organic.max(0.0)),
                    plastic: seed_round(split.plastic.max(0.0)),
                    paper: seed_round(split.paper.max(0.0)),
                    ewaste: seed_round(split.ewaste.max(0.0)),
                },
            });
        }
    }
    out
}

struct Totals {
    kwh: f64,
    water: f64,
    waste: f64,
    split: WasteSplit,
    occupant_days: i64,
}

fn totals_between(readings: &[Reading], from: &str, to: &str) -> Totals {
    let mut t = Totals {
        kwh: 0.0,
        water: 0.0,
        waste: 0.0,
        split: WasteSplit { organic: 0.0, plastic: 0.0, paper: 0.0, ewaste: 0.0 },
        occupant_days: 0,
    };
    for r in readings {
        if r.date < from || r.date > to {
            continue;
        }
        if let Some(b) = find_building(&r.building) {
            t.occupant_days += b.occupants as i64;
        }
        t.kwh += r.kwh;
        t.water += r.water_l;
        t.waste += r.waste_kg.total();
        t.split.organic += r.waste_kg.organic;
        t.split.plastic += r.waste_kg.plastic;
        t.split.paper += r.waste_kg.paper;
        t.split.ewaste += r.waste_kg.ewaste;
    }
    t
}

fn pct_change(cur: f64, prev: f64) -> f64 {
    if prev <= 0.0 {
        0.0
    } else {
        (cur - prev) / prev * 100.0
    }
}

pub fn compute_summary(readings: &[Reading], today: i64) -> Summary {
    let cur = totals_between(readings, &iso_date(today - 6), &iso_date(today));
    let prev = totals_between(readings, &iso_date(today - 13), &iso_date(today - 7));
    let base = totals_between(readings, &iso_date(today - 59), &iso_date(today));

    let subscore = |ratio: f64| {
        if ratio <= 0.0 {
            100.0
        } else {
            clamp(100.0 - 220.0 * (ratio - 0.88), 0.0, 100.0)
        }
    };
    let mut e_ratio = 1.0;
    let mut w_ratio = 1.0;
    let mut t_ratio = 1.0;
    if base.occupant_days > 0 && base.kwh > 0.0 && cur.occupant_days > 0 {
        e_ratio = (cur.kwh / cur.occupant_days as f64) / (base.kwh / base.occupant_days as f64);
    }
    if base.occupant_days > 0 && base.water > 0.0 && cur.occupant_days > 0 {
        w_ratio = (cur.water / cur.occupant_days as f64) / (base.water / base.occupant_days as f64);
    }
    if base.occupant_days > 0 && base.waste > 0.0 && cur.occupant_days > 0 {
        t_ratio = (cur.waste / cur.occupant_days as f64) / (base.waste / base.occupant_days as f64);
    }
    let score = 0.45 * subscore(e_ratio) + 0.30 * subscore(w_ratio) + 0.25 * subscore(t_ratio);

    Summary {
        window_days: 7,
        window_end: iso_date(today),
        kwh: cur.kwh,
        co2_kg: cur.kwh * CO2_PER_KWH,
        water_l: cur.water,
        waste_kg: cur.waste,
        waste_split: cur.split,
        change_pct: ChangePct {
            kwh: pct_change(cur.kwh, prev.kwh),
            co2_kg: pct_change(cur.kwh, prev.kwh),
            water_l: pct_change(cur.water, prev.water),
            waste_kg: pct_change(cur.waste, prev.waste),
        },
        campus_score: score.round().clamp(0.0, 100.0) as u32,
    }
}

pub fn compute_leaderboard(readings: &[Reading], today: i64) -> LeaderboardResponse {
    let from = iso_date(today - 6);
    let to = iso_date(today);
    let mut rows: Vec<LeaderRow> = BUILDINGS
        .iter()
        .map(|b| {
            let mut kwh = 0.0;
            for r in readings {
                if r.building == b.id && r.date >= from && r.date <= to {
                    kwh += r.kwh;
                }
            }
            let co2 = kwh * CO2_PER_KWH;
            LeaderRow {
                rank: 0,
                building: b.id.to_string(),
                label: b.label.to_string(),
                occupants: b.occupants,
                kwh,
                co2_kg: co2,
                co2_per_occupant: co2 / b.occupants as f64,
                score: 100,
            }
        })
        .collect();
    let mut best = f64::MAX;
    let mut worst = f64::MIN;
    for r in &rows {
        best = best.min(r.co2_per_occupant);
        worst = worst.max(r.co2_per_occupant);
    }
    for r in &mut rows {
        r.score = if worst - best < 1e-9 {
            100
        } else {
            (100.0 - 80.0 * (r.co2_per_occupant - best) / (worst - best))
                .round()
                .clamp(0.0, 100.0) as u32
        };
    }
    rows.sort_by(|a, b| {
        a.co2_per_occupant
            .partial_cmp(&b.co2_per_occupant)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.building.cmp(&b.building))
    });
    for (i, r) in rows.iter_mut().enumerate() {
        r.rank = (i + 1) as u32;
    }
    LeaderboardResponse { rows, generated_at: now_utc_iso() }
}

pub fn detect_alerts(readings: &[Reading], today: i64) -> AlertsResponse {
    let first = iso_date(today - 59);
    let last = iso_date(today);
    let mut by_building: Vec<(String, Vec<&Reading>)> = Vec::new();
    for r in readings {
        if r.date < first || r.date > last {
            continue;
        }
        match by_building.iter_mut().find(|(id, _)| *id == r.building) {
            Some((_, days)) => days.push(r),
            None => by_building.push((r.building.clone(), vec![r])),
        }
    }

    let metrics: [(&str, usize); 3] = [("kwh", 0), ("water_l", 0), ("waste_kg", 1)];
    let mut alerts: Vec<Alert> = Vec::new();

    for (id, days) in by_building.iter_mut() {
        days.sort_by(|a, b| a.date.cmp(&b.date));
        if days.len() < 9 {
            continue;
        }
        let label = find_building(id).map(|b| b.label).unwrap_or(id.as_str());
        for (metric, decimals) in metrics {
            let value_of = |r: &Reading| -> f64 {
                match metric {
                    "kwh" => r.kwh,
                    "water_l" => r.water_l,
                    _ => r.waste_kg.total(),
                }
            };
            for i in 7..days.len() {
                let mut sum = 0.0;
                let mut sum_sq = 0.0;
                for d in &days[i - 7..i] {
                    let v = value_of(d);
                    sum += v;
                    sum_sq += v * v;
                }
                let mean = sum / 7.0;
                let variance = sum_sq / 7.0 - mean * mean;
                let sigma = if variance > 0.0 { variance.sqrt() } else { 0.0 };
                let v = value_of(days[i]);
                if sigma <= 1e-9 || !(v > mean + 2.0 * sigma) || !(v > 1.40 * mean) {
                    continue;
                }
                let ratio = v / if mean > 0.0 { mean } else { 1.0 };
                let reason = match metric {
                    "kwh" => format!(
                        "Electricity at {label} hit {} kWh on {}, {:.1}x its rolling 7-day mean of {} kWh. Usual causes: HVAC running off-schedule, a meter fault, or equipment left on overnight.",
                        human_num(v, 0), days[i].date, ratio, human_num(mean, 0)
                    ),
                    "water_l" => format!(
                        "Water use at {label} reached {} L on {}, {:.1}x the rolling 7-day mean of {} L. On hostels, suspect a leak or an unscheduled laundry run; elsewhere, an open tap or a cooling loop.",
                        human_num(v, 0), days[i].date, ratio, human_num(mean, 0)
                    ),
                    _ => format!(
                        "Waste from {label} jumped to {} kg on {}, {:.1}x the rolling 7-day mean of {} kg. Could be a clearout, an event, or bins emptied after a gap.",
                        human_num(v, 1), days[i].date, ratio, human_num(mean, 1)
                    ),
                };
                alerts.push(Alert {
                    building: id.clone(),
                    label: label.to_string(),
                    date: days[i].date.clone(),
                    metric: metric.to_string(),
                    value: v,
                    mean,
                    sigma,
                    ratio_sigma: (v - mean) / sigma,
                    reason,
                });
            }
        }
    }

    alerts.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| a.building.cmp(&b.building))
    });
    alerts.truncate(12);
    AlertsResponse { count: alerts.len() as u64, alerts }
}

pub fn build_suggestions(readings: &[Reading], today: i64) -> SuggestionsResponse {
    #[derive(Default, Clone)]
    struct BStats {
        kwh7: f64,
        water7: f64,
        waste7: f64,
        plastic7: f64,
        ewaste7: f64,
        kwh_prev7: f64,
        kwh60: f64,
        water60: f64,
        occ_days7: i64,
        occ_days60: i64,
        per_occ_kwh7: f64,
        per_occ_water7: f64,
        plastic_share: f64,
    }

    let f7 = iso_date(today - 6);
    let fprev = iso_date(today - 13);
    let fend = iso_date(today - 7);
    let f60 = iso_date(today - 59);
    let today_s = iso_date(today);

    let mut stats: Vec<(String, BStats)> = BUILDINGS
        .iter()
        .map(|b| (b.id.to_string(), BStats::default()))
        .collect();

    for r in readings {
        let Some((_, s)) = stats.iter_mut().find(|(id, _)| *id == r.building) else {
            continue;
        };
        let occ = find_building(&r.building).map(|b| b.occupants as i64).unwrap_or(0);
        if r.date >= f7 && r.date <= today_s {
            s.kwh7 += r.kwh;
            s.water7 += r.water_l;
            s.waste7 += r.waste_kg.total();
            s.plastic7 += r.waste_kg.plastic;
            s.ewaste7 += r.waste_kg.ewaste;
            s.occ_days7 += occ;
        } else if r.date >= fprev && r.date <= fend {
            s.kwh_prev7 += r.kwh;
        }
        if r.date >= f60 && r.date <= today_s {
            s.kwh60 += r.kwh;
            s.water60 += r.water_l;
            s.occ_days60 += occ;
        }
    }

    let mut hostel_sum = 0.0;
    let mut hostel_count = 0;
    let mut campus_waste = 0.0;
    let mut campus_plastic = 0.0;
    for b in BUILDINGS {
        let Some((_, s)) = stats.iter_mut().find(|(id, _)| *id == b.id) else { continue };
        if s.occ_days7 > 0 {
            s.per_occ_kwh7 = s.kwh7 / s.occ_days7 as f64;
            s.per_occ_water7 = s.water7 / s.occ_days7 as f64;
        }
        if s.waste7 > 0.0 {
            s.plastic_share = s.plastic7 / s.waste7;
        }
        if b.kind == "hostel" {
            hostel_sum += s.per_occ_kwh7;
            hostel_count += 1;
        }
        campus_waste += s.waste7;
        campus_plastic += s.plastic7;
    }
    let hostel_avg = if hostel_count > 0 { hostel_sum / hostel_count as f64 } else { 0.0 };
    let campus_share = if campus_waste > 0.0 { campus_plastic / campus_waste } else { 0.0 };

    let mut out: Vec<Suggestion> = Vec::new();

    for b in BUILDINGS {
        let Some((id, s)) = stats.iter().find(|(id, _)| *id == b.id) else { continue };
        let mut energy_flagged = false;

        if b.kind == "hostel" && hostel_avg > 0.0 && s.per_occ_kwh7 > hostel_avg * 1.10 {
            energy_flagged = true;
            out.push(Suggestion {
                building: id.clone(),
                label: b.label.to_string(),
                category: "energy".into(),
                priority: "high".into(),
                text: format!(
                    "{} uses {}% more electricity per resident than the hostel average ({} vs {} kWh per person over the last 7 days). Late-night AC is the usual suspect: set thermostats to sleep mode after 23:00.",
                    b.label,
                    int_text((s.per_occ_kwh7 / hostel_avg - 1.0) * 100.0),
                    human_num(s.per_occ_kwh7, 1),
                    human_num(hostel_avg, 1)
                ),
            });
        }

        if s.occ_days60 > 0 && s.kwh60 > 0.0 && s.per_occ_kwh7 > 0.0 {
            let norm = s.kwh60 / s.occ_days60 as f64;
            if !energy_flagged && s.per_occ_kwh7 > norm * 1.10 {
                out.push(Suggestion {
                    building: id.clone(),
                    label: b.label.to_string(),
                    category: "energy".into(),
                    priority: "medium".into(),
                    text: format!(
                        "{} electricity is {}% above its own 60-day norm ({} vs {} kWh per person). Check schedules, timers and anything left running overnight.",
                        b.label,
                        int_text((s.per_occ_kwh7 / norm - 1.0) * 100.0),
                        human_num(s.per_occ_kwh7, 1),
                        human_num(norm, 1)
                    ),
                });
            }
        }

        if s.occ_days60 > 0 && s.water60 > 0.0 && s.per_occ_water7 > 0.0 {
            let norm = s.water60 / s.occ_days60 as f64;
            if s.per_occ_water7 > norm * 1.15 {
                out.push(Suggestion {
                    building: id.clone(),
                    label: b.label.to_string(),
                    category: "water".into(),
                    priority: if s.per_occ_water7 > norm * 1.30 { "high".into() } else { "medium".into() },
                    text: format!(
                        "Water at {} is {}% above its 60-day norm ({} vs {} L per person). Walk the building for drips, running cisterns and laundry overrides.",
                        b.label,
                        int_text((s.per_occ_water7 / norm - 1.0) * 100.0),
                        human_num(s.per_occ_water7, 0),
                        human_num(norm, 0)
                    ),
                });
            }
        }

        if s.plastic_share > campus_share + 0.05 {
            out.push(Suggestion {
                building: id.clone(),
                label: b.label.to_string(),
                category: "waste".into(),
                priority: "medium".into(),
                text: format!(
                    "Plastic is {}% of {}'s waste vs {}% campus-wide. Retrain the bin split; most of it comes from packaging.",
                    int_text(s.plastic_share * 100.0),
                    b.label,
                    int_text(campus_share * 100.0)
                ),
            });
        }

        if s.ewaste7 > 4.0 {
            out.push(Suggestion {
                building: id.clone(),
                label: b.label.to_string(),
                category: "ewaste".into(),
                priority: "high".into(),
                text: format!(
                    "{} has {} kg of e-waste logged this week. Book a certified pickup so it does not sit in stores.",
                    b.label,
                    human_num(s.ewaste7, 1)
                ),
            });
        }

        if s.kwh_prev7 > 0.0 && s.kwh7 > s.kwh_prev7 * 1.12 {
            out.push(Suggestion {
                building: id.clone(),
                label: b.label.to_string(),
                category: "energy".into(),
                priority: "medium".into(),
                text: format!(
                    "Electricity at {} is up {}% week-over-week. Worth a walk-through before it becomes the new normal.",
                    b.label,
                    int_text((s.kwh7 / s.kwh_prev7 - 1.0) * 100.0)
                ),
            });
        }
    }

    let lb = compute_leaderboard(readings, today);
    if let Some(best) = lb.rows.first() {
        out.push(Suggestion {
            building: best.building.clone(),
            label: best.label.clone(),
            category: "benchmark".into(),
            priority: "low".into(),
            text: format!(
                "{} is the campus benchmark right now at {} kg CO2 per person this week.",
                best.label,
                human_num(best.co2_per_occupant, 2)
            ),
        });
    }

    let prio_rank = |p: &str| match p {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    };
    out.sort_by(|a, b| {
        prio_rank(&a.priority)
            .cmp(&prio_rank(&b.priority))
            .then_with(|| a.building.cmp(&b.building))
    });
    out.truncate(8);
    SuggestionsResponse { suggestions: out }
}

pub fn generate() -> StoreSnapshot {
    let today = today_civil();
    let readings = seed_readings(today);
    let summary = compute_summary(&readings, today);
    let alerts = detect_alerts(&readings, today);
    let leaderboard = compute_leaderboard(&readings, today);
    let suggestions = build_suggestions(&readings, today);
    StoreSnapshot { summary, readings, alerts, leaderboard, suggestions }
}
