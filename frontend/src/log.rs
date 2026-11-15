use crate::api::{post_reading, Reading, StoredReading, WasteSplit, BUILDINGS};
use crate::demo::{iso_date, today_civil};
use crate::state::Store;
use crate::ui::{SectionHead, Skeleton};
use leptos::*;

#[component]
pub fn Log() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");

    let building = create_rw_signal("hostel-a".to_string());
    let date = create_rw_signal(iso_date(today_civil()));
    let kwh = create_rw_signal(String::new());
    let water = create_rw_signal(String::new());
    let organic = create_rw_signal(String::new());
    let plastic = create_rw_signal(String::new());
    let paper = create_rw_signal(String::new());
    let ewaste = create_rw_signal(String::new());
    let submitting = create_rw_signal(false);
    let error = create_rw_signal::<Option<String>>(None);
    let success = create_rw_signal::<Option<StoredReading>>(None);

    let today = iso_date(today_civil());

    let submit = move |e: ev::SubmitEvent| {
        e.prevent_default();
        error.set(None);
        let num = |s: String, field: &'static str| -> Result<f64, String> {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(format!("{field} is required."));
            }
            match trimmed.parse::<f64>() {
                Ok(v) if v.is_finite() && v >= 0.0 => Ok(v),
                Ok(_) => Err(format!("{field} must be zero or more.")),
                Err(_) => Err(format!("{field} must be a number.")),
            }
        };
        let parsed = (|| -> Result<Reading, String> {
            let kwh_v = num(kwh.get_untracked(), "Electricity (kWh)")?;
            let water_v = num(water.get_untracked(), "Water (litres)")?;
            let org_v = num(organic.get_untracked(), "Organic waste")?;
            let plas_v = num(plastic.get_untracked(), "Plastic waste")?;
            let pap_v = num(paper.get_untracked(), "Paper waste")?;
            let ew_v = num(ewaste.get_untracked(), "E-waste")?;
            let d = date.get_untracked().trim().to_string();
            if d.len() != 10 || d.as_bytes()[4] != b'-' || d.as_bytes()[7] != b'-' {
                return Err("Date must be formatted YYYY-MM-DD.".to_string());
            }
            Ok(Reading {
                building: building.get_untracked(),
                date: d,
                kwh: kwh_v,
                water_l: water_v,
                waste_kg: WasteSplit { organic: org_v, plastic: plas_v, paper: pap_v, ewaste: ew_v },
            })
        })();

        let reading = match parsed {
            Ok(r) => r,
            Err(msg) => {
                error.set(Some(msg));
                return;
            }
        };

        if crate::api::api_base().is_none() {
            error.set(Some("No backend is configured, so this reading cannot be filed. Set EMBER_API_URL when building the frontend.".to_string()));
            return;
        }

        submitting.set(true);
        let store_for_async = store.clone();
        spawn_local(async move {
            let store = store_for_async;
            match post_reading(&reading).await {
                Ok(stored) => {
                    let stored_reading = stored.stored.clone();
                    store.readings.update(|rs| {
                        rs.retain(|r| {
                            !(r.building == stored_reading.building && r.date == stored_reading.date)
                        });
                        rs.push(stored_reading.clone());
                        rs.sort_by(|a, b| {
                            a.building
                                .cmp(&b.building)
                                .then_with(|| a.date.cmp(&b.date))
                        });
                    });
                    success.set(Some(stored));
                    submitting.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("The ledger refused this reading: {e}")));
                    submitting.set(false);
                }
            }
        });
    };

    let reset = move |_| {
        success.set(None);
        kwh.set(String::new());
        water.set(String::new());
        organic.set(String::new());
        plastic.set(String::new());
        paper.set(String::new());
        ewaste.set(String::new());
        date.set(iso_date(today_civil()));
    };

    let field = |label: &'static str,
                 hint: &'static str,
                 sig: RwSignal<String>,
                 step: &'static str| {
        view! {
            <label class="field">
                <span class="field-label">{label}</span>
                <input
                    type="number"
                    min="0"
                    step=step
                    placeholder="0"
                    class="field-input"
                    value={sig.get_untracked()}
                    on:input=move |e| sig.set(event_target_value(&e))
                    required
                />
                <span class="field-hint">{hint}</span>
            </label>
        }
    };

    view! {
        <div class="log">
            <section class="section">
                <SectionHead
                    num="01"
                    title="File a reading"
                    note="one building · one day · five numbers off the meters"
                />
                {move || {
                    if let Some(stored) = success.get() {
                        let r = stored.stored.clone();
                        let verb = if stored.updated { "replaced" } else { "filed" };
                        let summary = format!(
                            "{} · {} · {} kWh · {} L water · {} kg waste",
                            r.building,
                            r.date,
                            r.kwh,
                            r.water_l,
                            r.waste_kg.total()
                        );
                        view! {
                            <div class="log-success" role="status">
                                <svg class="success-mark" viewBox="0 0 64 64" aria-hidden="true">
                                    <circle class="success-circle draw-line" pathLength="1000" cx="32" cy="32" r="26"/>
                                    <path
                                        class="success-check draw-line"
                                        pathLength="1000"
                                        style="--d: .5s"
                                        d="M 20 33 L 29 42 L 45 24"
                                    />
                                </svg>
                                <h2 class="success-title">
                                    <span class="mask"><span class="mask-inner">
                                        {format!("Reading {verb}.")}
                                    </span></span>
                                </h2>
                                <p class="success-summary">{summary}</p>
                                <p class="success-note">
                                    "The charts tween to it on the next poll; nothing else to do."
                                </p>
                                <button type="button" class="btn" on:click=reset>
                                    "File another reading"
                                </button>
                            </div>
                        }
                        .into_view()
                    } else if submitting.get() {
                        view! {
                            <div class="log-posting" role="status">
                                <p class="panel-title">filing to the ledger…</p>
                                <Skeleton lines=4/>
                            </div>
                        }
                        .into_view()
                    } else {
                        view! {
                            <form class="log-form" on:submit=submit novalidate>
                                <div class="log-grid">
                                    <label class="field">
                                        <span class="field-label">building</span>
                                        <select
                                            class="field-input"
                                            value={building.get_untracked()}
                                            on:change=move |e| building.set(event_target_value(&e))
                                        >
                                            {BUILDINGS
                                                .iter()
                                                .map(|b| {
                                                    let id = b.id.to_string();
                                                    let label = b.label.to_string();
                                                    view! {
                                                        <option value={id.clone()} selected={building.get_untracked() == id}>
                                                            {label}
                                                        </option>
                                                    }
                                                })
                                                .collect_view()}
                                        </select>
                                        <span class="field-hint">six buildings on the metering roll</span>
                                    </label>
                                    <label class="field">
                                        <span class="field-label">date</span>
                                        <input
                                            type="date"
                                            max={today}
                                            class="field-input"
                                            value={date.get_untracked()}
                                            on:input=move |e| date.set(event_target_value(&e))
                                            required
                                        />
                                        <span class="field-hint">the day the meters were read</span>
                                    </label>
                                    {field("electricity", "kWh for the whole day", kwh, "0.1")}
                                    {field("water", "litres for the whole day", water, "10")}
                                    {field("waste · organic", "kg", organic, "0.1")}
                                    {field("waste · plastic", "kg", plastic, "0.1")}
                                    {field("waste · paper", "kg", paper, "0.1")}
                                    {field("waste · e-waste", "kg", ewaste, "0.1")}
                                </div>
                                {move || {
                                    error.get().map(|msg| {
                                        view! {
                                            <div class="form-error" role="alert">{msg}</div>
                                        }
                                    })
                                }}
                                <div class="form-actions">
                                    <button type="submit" class="btn btn-primary">
                                        "File reading"
                                    </button>
                                    <span class="form-foot">
                                        "POST /readings · replaces an existing record for the same building and day"
                                    </span>
                                </div>
                            </form>
                        }
                        .into_view()
                    }
                }}
            </section>
        </div>
    }
}
