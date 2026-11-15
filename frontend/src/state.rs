use crate::api::*;
use crate::demo;
use leptos::*;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendStatus {
    Waking,
    Live,
    Demo,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub building: String,
    pub label: String,
    pub date: String,
    pub metric: String,
    pub reason: String,
}

#[derive(Clone, Copy)]
pub struct Store {
    pub status: RwSignal<BackendStatus>,
    pub waking_attempt: RwSignal<u32>,
    pub summary: RwSignal<Summary>,
    pub readings: RwSignal<Vec<Reading>>,
    pub alerts: RwSignal<AlertsResponse>,
    pub leaderboard: RwSignal<LeaderboardResponse>,
    pub suggestions: RwSignal<SuggestionsResponse>,
    pub toasts: RwSignal<Vec<Toast>>,
    pub seen_alerts: RwSignal<Vec<String>>,
    pub toast_seq: RwSignal<u64>,
    pub primed: RwSignal<bool>,
}

pub fn init_store() -> Store {
    let snapshot = demo::generate();
    let status = if api_base().is_some() { BackendStatus::Waking } else { BackendStatus::Demo };
    Store {
        status: create_rw_signal(status),
        waking_attempt: create_rw_signal(0),
        summary: create_rw_signal(snapshot.summary),
        readings: create_rw_signal(snapshot.readings),
        alerts: create_rw_signal(snapshot.alerts),
        leaderboard: create_rw_signal(snapshot.leaderboard),
        suggestions: create_rw_signal(snapshot.suggestions),
        toasts: create_rw_signal(Vec::new()),
        seen_alerts: create_rw_signal(Vec::new()),
        toast_seq: create_rw_signal(0),
        primed: create_rw_signal(false),
    }
}

async fn wait(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

fn push_new_alert_toasts(store: Store, resp: AlertsResponse) {
    let primed = store.primed.get_untracked();
    let mut seen = store.seen_alerts.get_untracked();
    let mut fresh: Vec<Alert> = Vec::new();
    for alert in &resp.alerts {
        let key = format!("{}|{}|{}", alert.building, alert.date, alert.metric);
        if !seen.iter().any(|k| k == &key) {
            seen.push(key);
            fresh.push(alert.clone());
        }
    }
    if seen.len() > 240 {
        let overflow = seen.len() - 240;
        seen.drain(0..overflow);
    }
    store.seen_alerts.set(seen);
    if !primed {
        store.primed.set(true);
        return;
    }
    if fresh.is_empty() {
        return;
    }
    store.toast_seq.update(|seq| {
        for alert in fresh {
            *seq += 1;
            let id = *seq;
            let toast = Toast {
                id,
                building: alert.building.clone(),
                label: alert.label.clone(),
                date: alert.date.clone(),
                metric: alert.metric.clone(),
                reason: alert.reason.clone(),
            };
            store.toasts.update(|toasts| toasts.push(toast));
            set_timeout(
                move || {
                    store.toasts.update(|toasts| toasts.retain(|t| t.id != id));
                },
                Duration::from_millis(7000),
            );
        }
    });
}

async fn refresh_all(store: Store) {
    let (s, a, l, g) = futures::join!(
        fetch_summary(),
        fetch_alerts(),
        fetch_leaderboard(),
        fetch_suggestions()
    );
    if let Ok(v) = s {
        store.summary.set(v);
    }
    if let Ok(v) = a {
        push_new_alert_toasts(store, v);
    }
    if let Ok(v) = l {
        store.leaderboard.set(v);
    }
    if let Ok(v) = g {
        store.suggestions.set(v);
    }
}

async fn refresh_readings(store: Store) {
    if let Ok(resp) = fetch_readings(60).await {
        if !resp.readings.is_empty() {
            store.readings.set(resp.readings);
        }
    }
}

pub fn start_backend(store: Store) {
    if api_base().is_none() {
        return;
    }
    spawn_local(async move {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            store.waking_attempt.set(attempt);
            match health(8000).await {
                Ok(_) => break,
                Err(_) => {
                    store.status.set(BackendStatus::Demo);
                    wait(9000).await;
                }
            }
        }
        store.status.set(BackendStatus::Live);
        refresh_all(store).await;
        refresh_readings(store).await;
        let mut ticks = 0u32;
        loop {
            wait(6000).await;
            refresh_all(store).await;
            ticks += 1;
            if ticks % 5 == 0 {
                refresh_readings(store).await;
            }
        }
    });
}
