# Ember API

The Ember backend is a C++17 service built with [cpp-httplib](https://github.com/yhirose/cpp-httplib). It keeps all data in memory, seeded at startup with a deterministic 60-day dataset for six campus buildings. The live service is deployed on Render; the frontend reads `EMBER_API_URL` (the Render URL) at compile time and talks to this API directly from the browser.

- All responses are `application/json; charset=utf-8`.
- CORS is open: every response carries `Access-Control-Allow-Origin: *`, `Access-Control-Allow-Methods: GET, POST, OPTIONS`, `Access-Control-Allow-Headers: Content-Type`, `Access-Control-Max-Age: 86400`. An `OPTIONS` request to any path returns `204 No Content`.
- Every error response has the same shape:

```json
{ "error": "a human-readable sentence" }
```

Status codes: `200` OK, `201` created, `400` validation error, `404` unknown route, `500` internal error.

- Render's free plan sleeps the service. The frontend pings `/health` with an 8 second timeout and renders a deterministic demo dataset while it waits; if `/health` never answers (or `EMBER_API_URL` is unset) the demo dataset stays on screen behind a small `demo data` tag.

---

## Data model

### Buildings

Six buildings are known to the server. `building` fields must be one of these ids. Occupant counts are used for per-capita metrics.

| id | label | type | occupants |
|---|---|---|---|
| `hostel-a` | Hostel A | hostel | 214 |
| `hostel-b` | Hostel B | hostel | 198 |
| `hostel-c` | Hostel C | hostel | 226 |
| `hostel-d` | Hostel D | hostel | 187 |
| `library` | Central Library | library | 410 |
| `main-block` | Main Academic Block | academic | 940 |

### Reading

One building's day. Dates are ISO `YYYY-MM-DD`, interpreted as UTC civil days.

```json
{
  "building": "hostel-c",
  "date": "2026-11-14",
  "kwh": 512.4,
  "water_l": 48300.0,
  "waste_kg": {
    "organic": 61.2,
    "plastic": 11.8,
    "paper": 14.2,
    "ewaste": 0.4
  }
}
```

| field | type | rules |
|---|---|---|
| `building` | string | must be a known building id |
| `date` | string | `YYYY-MM-DD`, a real calendar date, year 2000–2100 |
| `kwh` | number | finite, `>= 0`, `<= 50000` |
| `water_l` | number | finite, `>= 0`, `<= 2000000` |
| `waste_kg.organic` | number | finite, `>= 0`, `<= 20000` |
| `waste_kg.plastic` | number | finite, `>= 0`, `<= 20000` |
| `waste_kg.paper` | number | finite, `>= 0`, `<= 20000` |
| `waste_kg.ewaste` | number | finite, `>= 0`, `<= 20000` |

### Metric keys

Anywhere a metric is named (`/alerts`, change tables, chart series), the API uses exactly these keys:

- `kwh` — electricity, kWh/day
- `water_l` — water, litres/day
- `waste_kg` — total waste, kg/day (organic + plastic + paper + ewaste)

---

## Endpoints

### `GET /health`

Liveness probe. Confirms the process is up and the seed ran.

```json
{
  "ok": true,
  "service": "ember",
  "buildings": 6,
  "readings": 360,
  "uptime_s": 4271
}
```

`readings` is the number of stored day-records (360 right after boot: 6 buildings x 60 days). It grows if readings are filed for new dates.

---

### `POST /readings`

Files one daily reading. Body is a single JSON object with exactly the Reading shape above (all fields required, no extras tolerated beyond unknown keys being ignored).

Success — `201` when the (building, date) pair is new, `200` when it replaces an existing record for that pair. The response echoes the stored record.

```json
{
  "stored": {
    "building": "hostel-c",
    "date": "2026-11-14",
    "kwh": 512.4,
    "water_l": 48300.0,
    "waste_kg": { "organic": 61.2, "plastic": 11.8, "paper": 14.2, "ewaste": 0.4 }
  },
  "updated": false
}
```

Validation failures return `400` with a specific message:

| case | message |
|---|---|
| body is not JSON | `Body must be valid JSON.` |
| missing field | `Missing field: waste_kg.plastic` |
| unknown building | `Unknown building "hostel-z". Valid ids: hostel-a, hostel-b, hostel-c, hostel-d, library, main-block.` |
| bad date | `date must be a real calendar day formatted YYYY-MM-DD.` |
| negative / non-numeric | `kwh must be a non-negative number.` |

---

### `GET /readings?building=&days=`

Returns stored daily readings, newest last, sorted by building then date.

Query parameters, both optional:

- `building` — a building id to filter by; anything else is a `400` with the unknown-building message. Omitted or empty means all buildings.
- `days` — how many trailing days to return, `1` to `180`, default `60`. Non-integer or out of range is a `400` (`days must be an integer between 1 and 180.`).

```json
{
  "readings": [
    {
      "building": "hostel-a",
      "date": "2026-10-17",
      "kwh": 431.2,
      "water_l": 23140.0,
      "waste_kg": { "organic": 40.1, "plastic": 15.6, "paper": 17.9, "ewaste": 1.2 }
    }
  ],
  "count": 360
}
```

`count` is the number of readings in `readings`.

---

### `GET /summary`

Campus totals for the trailing 7 civil days (ending today, UTC), with the previous 7 days as the comparison window.

```json
{
  "window_days": 7,
  "window_end": "2026-11-14",
  "kwh": 21330.5,
  "co2_kg": 17491.0,
  "water_l": 1523800.0,
  "waste_kg": 2712.8,
  "waste_split": {
    "organic": 1487.2,
    "plastic": 542.6,
    "paper": 649.0,
    "ewaste": 34.0
  },
  "change_pct": {
    "kwh": 2.8,
    "co2_kg": 2.8,
    "water_l": 6.1,
    "waste_kg": -1.9
  },
  "campus_score": 61
}
```

- `co2_kg` = `kwh` x 0.82 (grid emission factor, kg CO2 per kWh).
- `change_pct` is `(this week - last week) / last week * 100` for each of `kwh`, `co2_kg`, `water_l`, `waste_kg`. If last week is zero, the change is reported as `0`.
- `waste_split` is the category split of `waste_kg` for the same trailing 7 days.

**Campus score (0–100).** Per-building efficiency over the last 7 days is compared against the same building's 60-day baseline, all normalised per occupant per day:

- `ratio = (7-day total / occupant-days in 7 days) / (60-day total / occupant-days in 60 days)` for each of energy (`kwh`), water (`water_l`), waste (`waste_kg`).
- each subscore = `clamp(0, 100, 100 - 220 * (ratio - 0.88))`: a week exactly on baseline scores about 74, 8% better than baseline saturates near 100, worse weeks fall away linearly.
- `campus_score = round(0.45 * energy + 0.30 * water + 0.25 * waste)`, clamped to 0–100.

---

### `GET /alerts`

Anomaly feed. For every building and every metric key (`kwh`, `water_l`, `waste_kg`), each day is compared against the rolling 7 days immediately before it:

- mean `m` and population standard deviation `s` of the previous 7 days,
- the day is an anomaly when `value > m + 2s` **and** `value > 1.40 * m` (the second clause keeps rolling-window noise and weekly patterns like laundry days from being flagged),
- at most the 12 most recent anomalies are returned, newest date first.

```json
{
  "alerts": [
    {
      "building": "hostel-d",
      "label": "Hostel D",
      "date": "2026-11-10",
      "metric": "water_l",
      "value": 91200.0,
      "mean": 44800.0,
      "sigma": 6900.0,
      "ratio_sigma": 6.7,
      "reason": "Water use at Hostel D reached 91,200 L on 2026-11-10, 2.0x its rolling 7-day mean of 44,800 L. On hostels, suspect a leak or an unscheduled laundry run; elsewhere, an open tap or cooling loop."
    }
  ],
  "count": 1
}
```

- `value` is the flagged day's value, `mean` and `sigma` come from the 7 days before it, `ratio_sigma` is `(value - mean) / sigma`.
- `reason` is a plain-language sentence naming the building, date, multiplier and the usual suspects for that metric. The frontend shows it verbatim.

---

### `GET /leaderboard`

Buildings ranked by electricity emissions per occupant over the trailing 7 days, most efficient first. `rank` 1 is the benchmark.

```json
{
  "rows": [
    {
      "rank": 1,
      "building": "library",
      "label": "Central Library",
      "occupants": 410,
      "kwh": 2032.0,
      "co2_kg": 1666.2,
      "co2_per_occupant": 4.06,
      "score": 100
    }
  ],
  "generated_at": "2026-11-15T06:02:11Z"
}
```

- `co2_kg` = trailing-7-day `kwh` x 0.82; `co2_per_occupant` = `co2_kg / occupants`.
- `score` (0–100) maps the building's per-capita position across campus: best building = 100, worst = 20, linear in between. Formula: `score = clamp(0, 100, 100 - 80 * (x - best) / (worst - best))`, all buildings score 100 in the degenerate case `worst == best`.

---

### `GET /suggestions`

Rule-based, building-specific advice generated server-side from the last 7 days against 60-day baselines and campus averages. Every number in every sentence is computed from stored readings, never hard-coded.

```json
{
  "suggestions": [
    {
      "building": "hostel-b",
      "label": "Hostel B",
      "category": "energy",
      "priority": "high",
      "text": "Hostel B uses 41% more electricity per resident than the hostel average (58.3 vs 41.2 kWh per person over the last 7 days). Late-night AC is the usual suspect: set thermostats to sleep mode after 23:00."
    }
  ]
}
```

- `category` is one of `energy`, `water`, `waste`, `ewaste`, `benchmark`.
- `priority` is `high`, `medium` or `low`; the list is sorted by priority then building, capped at 8 items.

The rules:

1. **Energy (high).** A hostel using >= 10% more electricity per resident than the hostel average: names the gap, points at late-night AC.
2. **Energy (medium).** A non-hostel (or hostel) using >= 10% more electricity per person than its own 60-day norm.
3. **Water (high if >= 30% above, else medium).** Any building >= 15% above its 60-day per-person water norm.
4. **Waste (medium).** Plastic share of a building's waste is >= 5 percentage points above the campus average share.
5. **E-waste (high).** Any building with more than 4 kg of e-waste logged this week: book a certified pickup.
6. **Trend (medium).** Any building whose electricity is up >= 12% week-over-week.
7. **Benchmark (low).** The current best per-capita building, so the leaderboard has a name worth copying.

---

## Seed behaviour

At boot the server generates 60 days ending today (UTC) for all six buildings, using a fixed PRNG seed, so every deployment starts with the same shape of data:

- **Hostels** carry a night-time AC signature: about 30% of days are "hot days" with ~17% higher electricity, plus weekend uplift (students in rooms all day). Water peaks midweek on laundry days (~25%).
- **Library and main academic block** drop hard on weekends and hold flat on weekdays.
- Waste is split per building type (hostels are organic-heavy, the academic block produces more paper and plastic).
- Five deterministic anomalies are injected so `/alerts` has material on first load: a two-day water leak in Hostel C (2.45x), an HVAC fault in the main block (1.72x kWh), a library event clearout (1.95x waste), a two-day dryer run in Hostel A (1.55x kWh), a recent laundry override in Hostel D (1.85x water), and a plastic contamination event in the main block (3.2x plastic).

The seed is regenerating every boot from the current date, so day labels stay current; the PRNG sequence, multipliers and injected anomalies are identical every boot.

`POST /readings` records persist only in memory. A Render free-tier redeploy or sleep-wake reseeds the base dataset; filed readings that overlapped the seeded window are replaced by the reseed.
