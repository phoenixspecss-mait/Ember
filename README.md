# Ember — Campus Energy & Waste Ledger

A campus energy and waste tracker: a C++17 REST API and a Rust/Leptos single-page dashboard over one shared ledger of meter readings. Six buildings, one shared ledger, seven-day analysis windows.

Built around **SDG 7** (affordable, clean energy), **SDG 11** (sustainable communities), **SDG 12** (responsible consumption), and **SDG 13** (climate action).

## Architecture

| Layer     | Stack                                    | Hosts on | Repo path  |
|-----------|------------------------------------------|----------|------------|
| Backend   | C++17, cpp-httplib (single header), CMake | Render   | `backend/` |
| Frontend  | Rust, Leptos 0.6 (CSR), Trunk, web-sys    | Vercel   | `frontend/`|
| Contract  | Full request/response reference           | —        | `API.md`   |

The backend is stateless apart from an in-memory ledger seeded at boot with 59 days of deterministic readings (fixed RNG seed, identical output on every cold start) for six buildings, ending yesterday so today's meters start open. All analytics — anomaly detection, scoring, suggestions — run server-side; the frontend mirrors the same formulas client-side only for its offline demo dataset.

```
Ember/
├── API.md                  full HTTP contract: shapes, formulas, seed behavior
├── render.yaml             Render Blueprint (Docker runtime, root dir backend)
└── backend/
    ├── main.cpp            http server, routing, CORS, validation, PORT env
    ├── analytics.cpp/.hpp  summary, alerts, leaderboard, suggestions
    ├── seed.hpp            deterministic 59-day, 6-building dataset
    ├── CMakeLists.txt
    └── Dockerfile
└── frontend/
    ├── Cargo.toml, Trunk.toml, index.html
    ├── build.sh            Vercel build: rustup + prebuilt trunk + trunk build
    ├── vercel.json         SPA rewrites, immutable asset caching
    ├── style.css           full design system (paper/ink/forest/vermillion)
    └── src/                app, api client, state/polling, pages, SVG charts
```

## API summary

All endpoints JSON; CORS enabled for every origin and method; invalid input returns `400` with `{"error": "..."}`.

| Endpoint | Purpose |
|---|---|
| `GET /health` | liveness + counts (buildings, readings, uptime) |
| `POST /readings` | file one building-day (kWh, litres, waste by category); replaces an existing row for the same building+date |
| `GET /readings?building=&days=` | reading history, 1–180 days, optional building filter |
| `GET /summary` | seven-day totals ending the latest complete day, week-over-week change, campus score |
| `GET /alerts` | anomaly days flagged by the rolling rule below, newest first, cap 12 |
| `GET /leaderboard` | per-building seven-day kWh, CO2, per-occupant CO2, 0–100 score |
| `GET /suggestions` | rule-based, building-specific action notes, priority-ordered, cap 8 |

Analytical windows end on the latest complete day: today once all six buildings have filed, otherwise yesterday. That keeps the seven-day comparisons like-for-like while today's meters are open.

**Formulas** (full detail in `API.md`):

- **CO2**: `0.82 kg per kWh`.
- **Alert rule**: a day is anomalous when `value > mean + 2σ` **and** `value > 1.40 × mean` of its rolling 7-day window (three metrics: electricity, water, total waste); the second clause keeps laundry-day and weekend rhythms from being flagged.
- **Leaderboard score**: best building = 100, worst = 20; linear in between, ranked by CO2 per occupant.
- **Campus score**: three subscores (`100 − 220 × (ratio − 0.88)` vs the 60-day per-occupant baseline) weighted 0.45 electricity / 0.30 water / 0.25 waste.
- **Suggestions**: hostels > 1.10× hostel average, any building > 1.10× its own 60-day norm, water > 1.15×/1.30× norm, plastic share > 5pt over campus, e-waste > 4 kg/week, electricity up > 1.12× week-over-week, plus a benchmark note for the leader.

## Frontend behavior

- `/` is a cover page: the ledger's current numbers on a plate, how the ledger works, what every formula means, and each building's state for today. "Get started" opens the dashboard at `/overview`.
- `/overview` is the dashboard, `/buildings` the per-building detail, `/log` the filing form.
- Today's meters start open: the cover page counts them, and the overview carries a "file now" callout until every building has a reading for the current day. Analytical windows close on the latest complete day, so today joins the week once all six readings are filed.
- The API base is baked in at compile time from `EMBER_API_URL` (`option_env!`). `frontend/build.sh` defaults it to `https://ember-9fux.onrender.com`, so Vercel builds are live without extra project settings; override the env var to point at another backend. A local build with no URL set runs entirely on the client-side demo dataset.
- With a URL set: it polls `/health` (8 s timeout) every 9 s until the backend wakes — the masthead shows a "waking the backend… try N" tag — then switches to live data, refreshing summary/alerts/leaderboard/suggestions every 6 s and full readings every 30 s. New anomalies arrive as toasts.
- Charts are hand-built SVG (line chart with anomaly markers and crosshair tooltip, score rings, waste bars with hatched fills, a score-driven tree illustration), animated with rAF tweens, and every animation respects `prefers-reduced-motion`.
- Responsive down to 360 px; leaderboard columns collapse on small screens.

## Deploying

**Backend on Render** — Blueprint deploy, no manual config:

1. Render → New → Blueprint → select this repo. `render.yaml` sets the Docker runtime, root dir `backend`, free plan, and health check `/health`.
2. No environment variables needed; `PORT` is injected by Render (defaults to 10000 locally).
3. Free instances sleep after ~15 min idle; the first request after a nap takes 30–50 s. The frontend's waking state covers this.

**Frontend on Vercel**:

1. Import the repo, set Root Directory to `frontend` (framework preset: Other).
2. Optional: set `EMBER_API_URL` to override the default Render backend (`https://ember-9fux.onrender.com`). It is a compile-time input — changing it requires a redeploy.
3. `vercel.json` supplies the build command (`bash ./build.sh`: installs Rust, adds the wasm target, fetches a prebuilt `trunk` binary, runs `trunk build --release`) and the `dist` output directory, SPA rewrites, and immutable caching for hashed `.js`/`.wasm` assets.

**Smoke test once both are live:**

```sh
TODAY=$(date -u +%F)
curl -X POST https://<render-url>/readings \
  -H 'Content-Type: application/json' \
  -d "{\"building\":\"hostel-a\",\"date\":\"$TODAY\",\"kwh\":900,\"water_l\":24000,\"waste_kg\":{\"organic\":40,\"plastic\":18,\"paper\":12,\"ewaste\":5}}"
```

The open-meter count drops immediately. Once all six buildings have a reading for today, the day closes, the seven-day window rolls forward, and the 5 kg e-waste raises a "book a certified pickup" field note.

## Local development

No toolchain setup is required to deploy, but both halves build conventionally:

```sh
cd backend && cmake -B build && cmake --build build && PORT=10000 ./build/ember-server
cd frontend && trunk serve --release    # add EMBER_API_URL=... for live data
```

`trunk serve` without `EMBER_API_URL` renders the full UI on the demo dataset — the same 59-day seed formulas the server uses, evaluated in the browser.
