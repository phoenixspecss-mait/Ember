use crate::api::api_base;
use crate::buildings::Buildings;
use crate::landing::Landing;
use crate::log::Log;
use crate::overview::Overview;
use crate::state::{init_store, start_backend, BackendStatus, Store};
use crate::ui::{Tag, Toasts};
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    let store = init_store();
    provide_context(store);
    start_backend(store);

    view! {
        <div class="page">
            <Router>
                <MastHead/>
                <main class="main">
                    <Routes>
                        <Route path="/" view=Landing/>
                        <Route path="/overview" view=Overview/>
                        <Route path="/buildings" view=Buildings/>
                        <Route path="/log" view=Log/>
                    </Routes>
                </main>
            </Router>
            <Footer/>
            <Toasts/>
        </div>
    }
}

#[component]
fn MastHead() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");
    let location = use_location();
    let is_active = move |path: &'static str| -> String {
        let current = location.pathname.get();
        if current == path || (!path.is_empty() && path != "/" && current.starts_with(path)) {
            "on".to_string()
        } else {
            String::new()
        }
    };
    let status = move || {
        let attempt = store.waking_attempt.get();
        match store.status.get() {
            BackendStatus::Live => {
                view! {
                    <Tag kind="live">
                        <span class="tag-dot"></span>
                        "live data"
                    </Tag>
                }
                .into_view()
            }
            BackendStatus::Waking => {
                view! {
                    <Tag kind="waking">
                        <span class="tag-dot"></span>
                        {format!("waking the backend… try {attempt}")}
                    </Tag>
                }
                .into_view()
            }
            BackendStatus::Demo => {
                let text = if api_base().is_some() {
                    "demo data · backend unreachable".to_string()
                } else {
                    "demo data".to_string()
                };
                view! {
                    <Tag kind="demo">{text}</Tag>
                }
                    .into_view()
            }
        }
    };
    view! {
        <header class="masthead">
            <div class="masthead-top">
                <a class="wordmark" href="/">
                    <span class="wordmark-mark" aria-hidden="true"></span>
                    Ember
                </a>
                <span class="masthead-sub">"campus energy & waste ledger"</span>
                <div class="masthead-status">{status}</div>
            </div>
            <nav class="nav" aria-label="Sections">
                <A href="/overview" class=move || is_active("/overview")>
                    <span class="nav-num">01</span> Overview
                </A>
                <A href="/buildings" class=move || is_active("/buildings")>
                    <span class="nav-num">02</span> Buildings
                </A>
                <A href="/log" class=move || is_active("/log")>
                    <span class="nav-num">03</span> Log reading
                </A>
            </nav>
            <div class="masthead-rule" aria-hidden="true"></div>
        </header>
    }
}

#[component]
fn Footer() -> impl IntoView {
    let store = use_context::<Store>().expect("store context");
    let source = move || match store.status.get() {
        BackendStatus::Live => "live readings from the metering point".to_string(),
        BackendStatus::Waking => "demo dataset while the backend wakes".to_string(),
        BackendStatus::Demo => "demo dataset".to_string(),
    };
    view! {
        <footer class="footer">
            <div class="footer-rule" aria-hidden="true"></div>
            <div class="footer-row">
                <span>{"Ember · SDG 7 · 11 · 12 · 13"}</span>
                <span class="footer-src">{source}</span>
                <span>0.82 kg CO2 per kWh</span>
            </div>
        </footer>
    }
}
