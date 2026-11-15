use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug)]
pub struct BuildingMeta {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub occupants: u32,
}

pub const BUILDINGS: [BuildingMeta; 6] = [
    BuildingMeta { id: "hostel-a", label: "Hostel A", kind: "hostel", occupants: 214 },
    BuildingMeta { id: "hostel-b", label: "Hostel B", kind: "hostel", occupants: 198 },
    BuildingMeta { id: "hostel-c", label: "Hostel C", kind: "hostel", occupants: 226 },
    BuildingMeta { id: "hostel-d", label: "Hostel D", kind: "hostel", occupants: 187 },
    BuildingMeta { id: "library", label: "Central Library", kind: "library", occupants: 410 },
    BuildingMeta { id: "main-block", label: "Main Academic Block", kind: "academic", occupants: 940 },
];

pub fn find_building(id: &str) -> Option<BuildingMeta> {
    BUILDINGS.iter().copied().find(|b| b.id == id)
}

pub fn is_hostel(id: &str) -> bool {
    matches!(find_building(id), Some(b) if b.kind == "hostel")
}

pub const CO2_PER_KWH: f64 = 0.82;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WasteSplit {
    pub organic: f64,
    pub plastic: f64,
    pub paper: f64,
    pub ewaste: f64,
}

impl WasteSplit {
    pub fn total(&self) -> f64 {
        self.organic + self.plastic + self.paper + self.ewaste
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reading {
    pub building: String,
    pub date: String,
    pub kwh: f64,
    pub water_l: f64,
    pub waste_kg: WasteSplit,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
    pub buildings: u32,
    pub readings: u64,
    pub uptime_s: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReadingsResponse {
    pub readings: Vec<Reading>,
    pub count: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChangePct {
    pub kwh: f64,
    pub co2_kg: f64,
    pub water_l: f64,
    pub waste_kg: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Summary {
    pub window_days: u32,
    pub window_end: String,
    pub kwh: f64,
    pub co2_kg: f64,
    pub water_l: f64,
    pub waste_kg: f64,
    pub waste_split: WasteSplit,
    pub change_pct: ChangePct,
    pub campus_score: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Alert {
    pub building: String,
    pub label: String,
    pub date: String,
    pub metric: String,
    pub value: f64,
    pub mean: f64,
    pub sigma: f64,
    pub ratio_sigma: f64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AlertsResponse {
    pub alerts: Vec<Alert>,
    pub count: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LeaderRow {
    pub rank: u32,
    pub building: String,
    pub label: String,
    pub occupants: u32,
    pub kwh: f64,
    pub co2_kg: f64,
    pub co2_per_occupant: f64,
    pub score: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LeaderboardResponse {
    pub rows: Vec<LeaderRow>,
    pub generated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Suggestion {
    pub building: String,
    pub label: String,
    pub category: String,
    pub priority: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SuggestionsResponse {
    pub suggestions: Vec<Suggestion>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StoredReading {
    pub stored: Reading,
    pub updated: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct ErrBody {
    error: String,
}

#[derive(Clone, Debug)]
pub enum ApiError {
    NoApi,
    Timeout,
    Network(String),
    Status(u16, String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NoApi => write!(f, "no backend configured"),
            ApiError::Timeout => write!(f, "backend did not answer in time"),
            ApiError::Network(m) => write!(f, "{m}"),
            ApiError::Status(_, m) => write!(f, "{m}"),
        }
    }
}

pub fn api_base() -> Option<&'static str> {
    static BASE: OnceLock<Option<String>> = OnceLock::new();
    BASE.get_or_init(|| {
        option_env!("EMBER_API_URL")
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
    })
    .as_deref()
}

async fn with_timeout<F: Future>(fut: F, ms: u32) -> Option<F::Output> {
    let fut = Box::pin(fut);
    let timer = Box::pin(gloo_timers::future::TimeoutFuture::new(ms));
    match futures::future::select(fut, timer).await {
        futures::future::Either::Left((out, _)) => Some(out),
        futures::future::Either::Right(_) => None,
    }
}

async fn get_json<T: for<'de> Deserialize<'de>>(path: &str, timeout_ms: u32) -> Result<T, ApiError> {
    let base = api_base().ok_or(ApiError::NoApi)?;
    let url = format!("{}{}", base, path);
    let resp = with_timeout(gloo_net::http::Request::get(&url).send(), timeout_ms)
        .await
        .ok_or(ApiError::Timeout)?
        .map_err(|e| ApiError::Network(e.to_string()))?;
    if !resp.ok() {
        let status = resp.status();
        let msg = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<ErrBody>(&msg)
            .ok()
            .map(|b| b.error)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(ApiError::Status(status, detail));
    }
    resp.json::<T>().await.map_err(|e| ApiError::Network(e.to_string()))
}

pub async fn health(timeout_ms: u32) -> Result<HealthResponse, ApiError> {
    get_json("/health", timeout_ms).await
}

pub async fn fetch_summary() -> Result<Summary, ApiError> {
    get_json("/summary", 8000).await
}

pub async fn fetch_alerts() -> Result<AlertsResponse, ApiError> {
    get_json("/alerts", 8000).await
}

pub async fn fetch_leaderboard() -> Result<LeaderboardResponse, ApiError> {
    get_json("/leaderboard", 8000).await
}

pub async fn fetch_suggestions() -> Result<SuggestionsResponse, ApiError> {
    get_json("/suggestions", 8000).await
}

pub async fn fetch_readings(days: u32) -> Result<ReadingsResponse, ApiError> {
    get_json(&format!("/readings?days={days}"), 10000).await
}

pub async fn post_reading(reading: &Reading) -> Result<StoredReading, ApiError> {
    let base = api_base().ok_or(ApiError::NoApi)?;
    let url = format!("{base}/readings");
    let request = gloo_net::http::Request::post(&url)
        .json(reading)
        .map_err(|e| ApiError::Network(e.to_string()))?;
    let resp = with_timeout(request.send(), 10000)
        .await
        .ok_or(ApiError::Timeout)?
        .map_err(|e| ApiError::Network(e.to_string()))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !resp.ok() {
        let detail = serde_json::from_str::<ErrBody>(&text)
            .ok()
            .map(|b| b.error)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(ApiError::Status(status, detail));
    }
    serde_json::from_str::<StoredReading>(&text).map_err(|e| ApiError::Network(e.to_string()))
}

#[derive(Clone, Debug)]
pub struct StoreSnapshot {
    pub summary: Summary,
    pub readings: Vec<Reading>,
    pub alerts: AlertsResponse,
    pub leaderboard: LeaderboardResponse,
    pub suggestions: SuggestionsResponse,
}
