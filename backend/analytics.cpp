#include "analytics.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <map>
#include <sstream>
#include <string>
#include <vector>

namespace ember {

static const std::vector<Building> kBuildings = {
    {"hostel-a", "Hostel A", "hostel", 214},
    {"hostel-b", "Hostel B", "hostel", 198},
    {"hostel-c", "Hostel C", "hostel", 226},
    {"hostel-d", "Hostel D", "hostel", 187},
    {"library", "Central Library", "library", 410},
    {"main-block", "Main Academic Block", "academic", 940},
};

const std::vector<Building>& buildings() { return kBuildings; }

const Building* find_building(const std::string& id) {
  for (const Building& b : kBuildings) {
    if (b.id == id) return &b;
  }
  return nullptr;
}

bool is_hostel(const std::string& id) {
  const Building* b = find_building(id);
  return b != nullptr && b->type == "hostel";
}

double co2_from_kwh(double kwh) { return kwh * 0.82; }

long days_from_civil(int y, unsigned m, unsigned d) {
  y -= m <= 2;
  const long era = (y >= 0 ? y : y - 399) / 400;
  const unsigned yoe = static_cast<unsigned>(y - era * 400);
  const unsigned mp = static_cast<unsigned>(m) + (m > 2 ? -3u : 9u);
  const unsigned doy = (153u * mp + 2u) / 5u + d - 1u;
  const unsigned doe = yoe * 365u + yoe / 4u - yoe / 100u + doy;
  return era * 146097L + static_cast<long>(doe) - 719468L;
}

long today_civil() {
  std::time_t t = std::time(nullptr);
  std::tm tm{};
#if defined(_WIN32)
  gmtime_s(&tm, &t);
#else
  gmtime_r(&t, &tm);
#endif
  return days_from_civil(tm.tm_year + 1900, static_cast<unsigned>(tm.tm_mon + 1),
                         static_cast<unsigned>(tm.tm_mday));
}

std::string iso_date(long civil_day) {
  long z = civil_day + 719468;
  const long era = (z >= 0 ? z : z - 146096) / 146097;
  const unsigned doe = static_cast<unsigned>(z - era * 146097);
  const unsigned yoe = (doe - doe / 1460u + doe / 36524u - doe / 146096u) / 365u;
  const long y = static_cast<long>(yoe) + era * 400L;
  const unsigned doy = doe - (365u * yoe + yoe / 4u - yoe / 100u);
  const unsigned mp = (5u * doy + 2u) / 153u;
  const unsigned d = doy - (153u * mp + 2u) / 5u + 1u;
  const unsigned m = mp + (mp < 10u ? 3u : -9u);
  const long fy = y + static_cast<long>(m <= 2);
  char buf[16];
  std::snprintf(buf, sizeof(buf), "%04ld-%02u-%02u", fy, m, d);
  return std::string(buf);
}

static std::string now_utc_iso() {
  std::time_t t = std::time(nullptr);
  std::tm tm{};
#if defined(_WIN32)
  gmtime_s(&tm, &t);
#else
  gmtime_r(&t, &tm);
#endif
  char buf[32];
  std::strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%SZ", &tm);
  return std::string(buf);
}

double WasteSplit::total() const { return organic + plastic + paper + ewaste; }

static double clampd(double v, double lo, double hi) {
  return v < lo ? lo : (v > hi ? hi : v);
}

std::string json_escape(const std::string& s) {
  std::string out;
  out.reserve(s.size() + 8);
  for (char c : s) {
    const unsigned char u = static_cast<unsigned char>(c);
    if (c == '"') {
      out += "\\\"";
    } else if (c == '\\') {
      out += "\\\\";
    } else if (u < 0x20) {
      char buf[8];
      std::snprintf(buf, sizeof(buf), "\\u%04x", u);
      out += buf;
    } else {
      out += c;
    }
  }
  return out;
}

std::string json_num(double v) {
  if (!std::isfinite(v)) return "0";
  if (std::fabs(v) < 0.005) return "0";
  char buf[64];
  std::snprintf(buf, sizeof(buf), "%.2f", v);
  std::string s(buf);
  if (s.find('.') != std::string::npos) {
    while (!s.empty() && s.back() == '0') s.pop_back();
    if (!s.empty() && s.back() == '.') s.pop_back();
  }
  if (s == "-0") s = "0";
  return s;
}

static std::string human_num(double v, int decimals) {
  char buf[80];
  std::snprintf(buf, sizeof(buf), "%.*f", decimals, v);
  std::string s(buf);
  std::size_t dot = s.find('.');
  std::size_t end = dot == std::string::npos ? s.size() : dot;
  std::size_t start = 0;
  if (!s.empty() && (s[0] == '-' || s[0] == '+')) start = 1;
  if (end > start + 3) {
    std::string grouped;
    const std::string head = s.substr(0, start);
    const std::string digits = s.substr(start, end - start);
    const std::string tail = dot == std::string::npos ? "" : s.substr(dot);
    for (std::size_t i = 0; i < digits.size(); ++i) {
      if (i > 0 && (digits.size() - i) % 3 == 0) grouped += ',';
      grouped += digits[i];
    }
    s = head + grouped + tail;
  }
  return s;
}

static std::string int_text(double v) {
  char buf[32];
  std::snprintf(buf, sizeof(buf), "%.0f", v);
  return std::string(buf);
}

std::string json_reading(const Reading& r) {
  std::ostringstream o;
  o << "{\"building\":\"" << json_escape(r.building) << "\",\"date\":\"" << json_escape(r.date)
    << "\",\"kwh\":" << json_num(r.kwh) << ",\"water_l\":" << json_num(r.water_l)
    << ",\"waste_kg\":{\"organic\":" << json_num(r.waste.organic)
    << ",\"plastic\":" << json_num(r.waste.plastic) << ",\"paper\":" << json_num(r.waste.paper)
    << ",\"ewaste\":" << json_num(r.waste.ewaste) << "}}";
  return o.str();
}

std::vector<Reading> select_readings(const std::vector<Reading>& all, long today,
                                     const std::string& building, int days) {
  const std::string first = iso_date(today - (days - 1));
  const std::string last = iso_date(today);
  std::vector<Reading> out;
  for (const Reading& r : all) {
    if (!building.empty() && building != "all" && r.building != building) continue;
    if (r.date < first || r.date > last) continue;
    out.push_back(r);
  }
  std::sort(out.begin(), out.end(), [](const Reading& a, const Reading& b) {
    if (a.building != b.building) return a.building < b.building;
    return a.date < b.date;
  });
  return out;
}

struct WindowTotals {
  double kwh = 0.0;
  double water = 0.0;
  double waste = 0.0;
  WasteSplit split;
  long occupant_days = 0;
};

static WindowTotals totals_between(const std::vector<Reading>& all, const std::string& from,
                                   const std::string& to) {
  WindowTotals t;
  for (const Reading& r : all) {
    if (r.date < from || r.date > to) continue;
    const Building* b = find_building(r.building);
    if (b != nullptr) t.occupant_days += b->occupants;
    t.kwh += r.kwh;
    t.water += r.water_l;
    t.waste += r.waste.total();
    t.split.organic += r.waste.organic;
    t.split.plastic += r.waste.plastic;
    t.split.paper += r.waste.paper;
    t.split.ewaste += r.waste.ewaste;
  }
  return t;
}

static double pct_change(double cur, double prev) {
  if (prev <= 0.0) return 0.0;
  return (cur - prev) / prev * 100.0;
}

std::string json_summary(const Summary& s) {
  std::ostringstream o;
  o << "{\"window_days\":" << s.window_days << ",\"window_end\":\"" << json_escape(s.window_end)
    << "\",\"kwh\":" << json_num(s.kwh) << ",\"co2_kg\":" << json_num(s.co2_kg)
    << ",\"water_l\":" << json_num(s.water_l) << ",\"waste_kg\":" << json_num(s.waste_kg)
    << ",\"waste_split\":{\"organic\":" << json_num(s.waste_split.organic)
    << ",\"plastic\":" << json_num(s.waste_split.plastic) << ",\"paper\":" << json_num(s.waste_split.paper)
    << ",\"ewaste\":" << json_num(s.waste_split.ewaste) << "}"
    << ",\"change_pct\":{\"kwh\":" << json_num(s.change_pct_kwh)
    << ",\"co2_kg\":" << json_num(s.change_pct_co2) << ",\"water_l\":" << json_num(s.change_pct_water)
    << ",\"waste_kg\":" << json_num(s.change_pct_waste) << "}"
    << ",\"campus_score\":" << s.campus_score << "}";
  return o.str();
}

Summary compute_summary(const std::vector<Reading>& all, long today) {
  Summary s;
  s.window_end = iso_date(today);

  const WindowTotals cur = totals_between(all, iso_date(today - 6), iso_date(today));
  const WindowTotals prev = totals_between(all, iso_date(today - 13), iso_date(today - 7));
  const WindowTotals base = totals_between(all, iso_date(today - 59), iso_date(today));

  s.kwh = cur.kwh;
  s.co2_kg = co2_from_kwh(cur.kwh);
  s.water_l = cur.water;
  s.waste_kg = cur.waste;
  s.waste_split = cur.split;
  s.change_pct_kwh = pct_change(cur.kwh, prev.kwh);
  s.change_pct_co2 = s.change_pct_kwh;
  s.change_pct_water = pct_change(cur.water, prev.water);
  s.change_pct_waste = pct_change(cur.waste, prev.waste);

  auto subscore = [](double ratio) {
    if (ratio <= 0.0) return 100.0;
    return clampd(100.0 - 220.0 * (ratio - 0.88), 0.0, 100.0);
  };
  double e_ratio = 1.0;
  double w_ratio = 1.0;
  double t_ratio = 1.0;
  if (base.occupant_days > 0 && base.kwh > 0.0 && cur.occupant_days > 0) {
    e_ratio = (cur.kwh / static_cast<double>(cur.occupant_days)) /
              (base.kwh / static_cast<double>(base.occupant_days));
  }
  if (base.occupant_days > 0 && base.water > 0.0 && cur.occupant_days > 0) {
    w_ratio = (cur.water / static_cast<double>(cur.occupant_days)) /
              (base.water / static_cast<double>(base.occupant_days));
  }
  if (base.occupant_days > 0 && base.waste > 0.0 && cur.occupant_days > 0) {
    t_ratio = (cur.waste / static_cast<double>(cur.occupant_days)) /
              (base.waste / static_cast<double>(base.occupant_days));
  }
  const double score = 0.45 * subscore(e_ratio) + 0.30 * subscore(w_ratio) + 0.25 * subscore(t_ratio);
  s.campus_score = static_cast<int>(std::lround(clampd(score, 0.0, 100.0)));
  return s;
}

std::vector<Alert> detect_alerts(const std::vector<Reading>& all, long today) {
  std::map<std::string, std::vector<const Reading*>> by_building;
  const std::string first = iso_date(today - 59);
  const std::string last = iso_date(today);
  for (const Reading& r : all) {
    if (r.date < first || r.date > last) continue;
    by_building[r.building].push_back(&r);
  }

  struct MetricDef {
    const char* key;
    int decimals;
  };
  const std::vector<MetricDef> metrics = {{"kwh", 0}, {"water_l", 0}, {"waste_kg", 1}};

  std::vector<Alert> alerts;
  for (auto& entry : by_building) {
    std::vector<const Reading*>& days = entry.second;
    std::sort(days.begin(), days.end(),
              [](const Reading* a, const Reading* b) { return a->date < b->date; });
    if (days.size() < 9) continue;
    const Building* b = find_building(entry.first);
    const std::string label = b != nullptr ? b->label : entry.first;

    for (const MetricDef& m : metrics) {
      auto value_of = [&](const Reading* r) -> double {
        if (m.key == std::string("kwh")) return r->kwh;
        if (m.key == std::string("water_l")) return r->water_l;
        return r->waste.total();
      };
      for (std::size_t i = 7; i < days.size(); ++i) {
        double sum = 0.0;
        double sum_sq = 0.0;
        for (std::size_t j = i - 7; j < i; ++j) {
          const double v = value_of(days[j]);
          sum += v;
          sum_sq += v * v;
        }
        const double mean = sum / 7.0;
        const double variance = sum_sq / 7.0 - mean * mean;
        const double sigma = variance > 0.0 ? std::sqrt(variance) : 0.0;
        const double v = value_of(days[i]);
        if (sigma <= 1e-9) continue;
        if (!(v > mean + 2.0 * sigma)) continue;
        if (!(v > 1.40 * mean)) continue;

        Alert a;
        a.building = entry.first;
        a.label = label;
        a.date = days[i]->date;
        a.metric = m.key;
        a.value = v;
        a.mean = mean;
        a.sigma = sigma;
        a.ratio_sigma = (v - mean) / sigma;

        const std::string vtxt = human_num(v, m.decimals);
        const std::string mtxt = human_num(mean, m.decimals);
        char ratio_buf[16];
        std::snprintf(ratio_buf, sizeof(ratio_buf), "%.1f", v / (mean > 0.0 ? mean : 1.0));
        const std::string ratio = ratio_buf;
        if (m.key == std::string("kwh")) {
          a.reason = "Electricity at " + label + " hit " + vtxt + " kWh on " + a.date + ", " +
                     ratio + "x its rolling 7-day mean of " + mtxt +
                     " kWh. Usual causes: HVAC running off-schedule, a meter fault, or equipment left on overnight.";
        } else if (m.key == std::string("water_l")) {
          a.reason = "Water use at " + label + " reached " + vtxt + " L on " + a.date + ", " +
                     ratio + "x the rolling 7-day mean of " + mtxt +
                     " L. On hostels, suspect a leak or an unscheduled laundry run; elsewhere, an open tap or a cooling loop.";
        } else {
          a.reason = "Waste from " + label + " jumped to " + vtxt + " kg on " + a.date + ", " +
                     ratio + "x the rolling 7-day mean of " + mtxt +
                     " kg. Could be a clearout, an event, or bins emptied after a gap.";
        }
        alerts.push_back(a);
      }
    }
  }

  std::sort(alerts.begin(), alerts.end(), [](const Alert& a, const Alert& b) {
    if (a.date != b.date) return a.date > b.date;
    return a.building < b.building;
  });
  if (alerts.size() > 12) alerts.resize(12);
  return alerts;
}

std::string json_alerts(const std::vector<Alert>& alerts) {
  std::ostringstream o;
  o << "{\"alerts\":[";
  for (std::size_t i = 0; i < alerts.size(); ++i) {
    const Alert& a = alerts[i];
    if (i > 0) o << ",";
    o << "{\"building\":\"" << json_escape(a.building) << "\",\"label\":\"" << json_escape(a.label)
      << "\",\"date\":\"" << json_escape(a.date) << "\",\"metric\":\"" << json_escape(a.metric)
      << "\",\"value\":" << json_num(a.value) << ",\"mean\":" << json_num(a.mean)
      << ",\"sigma\":" << json_num(a.sigma) << ",\"ratio_sigma\":" << json_num(a.ratio_sigma)
      << ",\"reason\":\"" << json_escape(a.reason) << "\"}";
  }
  o << "],\"count\":" << alerts.size() << "}";
  return o.str();
}

Leaderboard compute_leaderboard(const std::vector<Reading>& all, long today) {
  const std::string from = iso_date(today - 6);
  const std::string to = iso_date(today);
  std::vector<LeaderRow> rows;
  for (const Building& b : kBuildings) {
    LeaderRow row;
    row.building = b.id;
    row.label = b.label;
    row.occupants = b.occupants;
    for (const Reading& r : all) {
      if (r.building != b.id || r.date < from || r.date > to) continue;
      row.kwh += r.kwh;
    }
    row.co2_kg = co2_from_kwh(row.kwh);
    row.co2_per_occupant = row.occupants > 0 ? row.co2_kg / static_cast<double>(b.occupants) : 0.0;
    rows.push_back(row);
  }

  double best = 0.0;
  double worst = 0.0;
  bool first = true;
  for (const LeaderRow& r : rows) {
    if (first) {
      best = r.co2_per_occupant;
      worst = r.co2_per_occupant;
      first = false;
    } else {
      best = std::min(best, r.co2_per_occupant);
      worst = std::max(worst, r.co2_per_occupant);
    }
  }
  for (LeaderRow& r : rows) {
    if (worst - best < 1e-9) {
      r.score = 100;
    } else {
      r.score = static_cast<int>(
          std::lround(clampd(100.0 - 80.0 * (r.co2_per_occupant - best) / (worst - best), 0.0, 100.0)));
    }
  }

  std::sort(rows.begin(), rows.end(), [](const LeaderRow& a, const LeaderRow& b) {
    if (a.co2_per_occupant != b.co2_per_occupant) return a.co2_per_occupant < b.co2_per_occupant;
    return a.building < b.building;
  });
  for (std::size_t i = 0; i < rows.size(); ++i) rows[i].rank = static_cast<int>(i + 1);

  Leaderboard lb;
  lb.rows = rows;
  lb.generated_at = now_utc_iso();
  return lb;
}

std::string json_leaderboard(const Leaderboard& lb) {
  std::ostringstream o;
  o << "{\"rows\":[";
  for (std::size_t i = 0; i < lb.rows.size(); ++i) {
    const LeaderRow& r = lb.rows[i];
    if (i > 0) o << ",";
    o << "{\"rank\":" << r.rank << ",\"building\":\"" << json_escape(r.building)
      << "\",\"label\":\"" << json_escape(r.label) << "\",\"occupants\":" << r.occupants
      << ",\"kwh\":" << json_num(r.kwh) << ",\"co2_kg\":" << json_num(r.co2_kg)
      << ",\"co2_per_occupant\":" << json_num(r.co2_per_occupant) << ",\"score\":" << r.score << "}";
  }
  o << "],\"generated_at\":\"" << json_escape(lb.generated_at) << "\"}";
  return o.str();
}

std::vector<Suggestion> build_suggestions(const std::vector<Reading>& all, long today) {
  struct BStats {
    double kwh7 = 0.0;
    double water7 = 0.0;
    double waste7 = 0.0;
    double plastic7 = 0.0;
    double ewaste7 = 0.0;
    double kwh_prev7 = 0.0;
    double kwh60 = 0.0;
    double water60 = 0.0;
    long occ_days7 = 0;
    long occ_days60 = 0;
    double per_occ_kwh7 = 0.0;
    double per_occ_water7 = 0.0;
    double plastic_share = 0.0;
  };
  const std::string f7 = iso_date(today - 6);
  const std::string fprev = iso_date(today - 13);
  const std::string fend = iso_date(today - 7);
  const std::string f60 = iso_date(today - 59);

  std::map<std::string, BStats> stats;
  for (const Building& b : kBuildings) stats[b.id];

  for (const Reading& r : all) {
    auto it = stats.find(r.building);
    if (it == stats.end()) continue;
    BStats& s = it->second;
    const Building* b = find_building(r.building);
    const int occ = b != nullptr ? b->occupants : 0;
    if (r.date >= f7 && r.date <= iso_date(today)) {
      s.kwh7 += r.kwh;
      s.water7 += r.water_l;
      s.waste7 += r.waste.total();
      s.plastic7 += r.waste.plastic;
      s.ewaste7 += r.waste.ewaste;
      s.occ_days7 += occ;
    } else if (r.date >= fprev && r.date <= fend) {
      s.kwh_prev7 += r.kwh;
    }
    if (r.date >= f60 && r.date <= iso_date(today)) {
      s.kwh60 += r.kwh;
      s.water60 += r.water_l;
      s.occ_days60 += occ;
    }
  }

  double hostel_per_occ_sum = 0.0;
  int hostel_count = 0;
  double campus_waste7 = 0.0;
  double campus_plastic7 = 0.0;
  for (const Building& b : kBuildings) {
    BStats& s = stats[b.id];
    if (s.occ_days7 > 0) {
      s.per_occ_kwh7 = s.kwh7 / static_cast<double>(s.occ_days7);
      s.per_occ_water7 = s.water7 / static_cast<double>(s.occ_days7);
    }
    if (s.waste7 > 0.0) s.plastic_share = s.plastic7 / s.waste7;
    if (b.type == "hostel") {
      hostel_per_occ_sum += s.per_occ_kwh7;
      hostel_count += 1;
    }
    campus_waste7 += s.waste7;
    campus_plastic7 += s.plastic7;
  }
  const double hostel_avg = hostel_count > 0 ? hostel_per_occ_sum / hostel_count : 0.0;
  const double campus_plastic_share = campus_waste7 > 0.0 ? campus_plastic7 / campus_waste7 : 0.0;

  std::vector<Suggestion> out;
  bool energy_flagged = false;

  for (const Building& b : kBuildings) {
    const BStats& s = stats[b.id];

    if (b.type == "hostel" && hostel_avg > 0.0 && s.per_occ_kwh7 > hostel_avg * 1.10) {
      Suggestion sug;
      sug.building = b.id;
      sug.label = b.label;
      sug.category = "energy";
      sug.priority = "high";
      const int pct = static_cast<int>(std::lround((s.per_occ_kwh7 / hostel_avg - 1.0) * 100.0));
      sug.text = b.label + " uses " + int_text(pct) +
                 "% more electricity per resident than the hostel average (" +
                 human_num(s.per_occ_kwh7, 1) + " vs " + human_num(hostel_avg, 1) +
                 " kWh per person over the last 7 days). Late-night AC is the usual suspect: set thermostats to sleep mode after 23:00.";
      out.push_back(sug);
      energy_flagged = true;
    }

    if (s.occ_days60 > 0 && s.kwh60 > 0.0 && s.per_occ_kwh7 > 0.0) {
      const double norm = s.kwh60 / static_cast<double>(s.occ_days60);
      if (!energy_flagged && s.per_occ_kwh7 > norm * 1.10) {
        Suggestion sug;
        sug.building = b.id;
        sug.label = b.label;
        sug.category = "energy";
        sug.priority = "medium";
        const int pct = static_cast<int>(std::lround((s.per_occ_kwh7 / norm - 1.0) * 100.0));
        sug.text = b.label + " electricity is " + int_text(pct) + "% above its own 60-day norm (" +
                   human_num(s.per_occ_kwh7, 1) + " vs " + human_num(norm, 1) +
                   " kWh per person). Check schedules, timers and anything left running overnight.";
        out.push_back(sug);
      }
    }
    energy_flagged = false;

    if (s.occ_days60 > 0 && s.water60 > 0.0 && s.per_occ_water7 > 0.0) {
      const double norm = s.water60 / static_cast<double>(s.occ_days60);
      if (s.per_occ_water7 > norm * 1.15) {
        Suggestion sug;
        sug.building = b.id;
        sug.label = b.label;
        sug.category = "water";
        sug.priority = s.per_occ_water7 > norm * 1.30 ? "high" : "medium";
        const int pct = static_cast<int>(std::lround((s.per_occ_water7 / norm - 1.0) * 100.0));
        sug.text = "Water at " + b.label + " is " + int_text(pct) + "% above its 60-day norm (" +
                   human_num(s.per_occ_water7, 0) + " vs " + human_num(norm, 0) +
                   " L per person). Walk the building for drips, running cisterns and laundry overrides.";
        out.push_back(sug);
      }
    }

    if (s.plastic_share > campus_plastic_share + 0.05) {
      Suggestion sug;
      sug.building = b.id;
      sug.label = b.label;
      sug.category = "waste";
      sug.priority = "medium";
      const int share_b = static_cast<int>(std::lround(s.plastic_share * 100.0));
      const int share_c = static_cast<int>(std::lround(campus_plastic_share * 100.0));
      sug.text = "Plastic is " + int_text(share_b) + "% of " + b.label + "'s waste vs " +
                 int_text(share_c) +
                 "% campus-wide. Retrain the bin split; most of it comes from packaging.";
      out.push_back(sug);
    }

    if (s.ewaste7 > 4.0) {
      Suggestion sug;
      sug.building = b.id;
      sug.label = b.label;
      sug.category = "ewaste";
      sug.priority = "high";
      sug.text = b.label + " has " + human_num(s.ewaste7, 1) +
                 " kg of e-waste logged this week. Book a certified pickup so it does not sit in stores.";
      out.push_back(sug);
    }

    if (s.kwh_prev7 > 0.0 && s.kwh7 > s.kwh_prev7 * 1.12) {
      Suggestion sug;
      sug.building = b.id;
      sug.label = b.label;
      sug.category = "energy";
      sug.priority = "medium";
      const int pct = static_cast<int>(std::lround((s.kwh7 / s.kwh_prev7 - 1.0) * 100.0));
      sug.text = "Electricity at " + b.label + " is up " + int_text(pct) +
                 "% week-over-week. Worth a walk-through before it becomes the new normal.";
      out.push_back(sug);
    }
  }

  const Leaderboard lb = compute_leaderboard(all, today);
  if (!lb.rows.empty()) {
    const LeaderRow& best = lb.rows.front();
    Suggestion sug;
    sug.building = best.building;
    sug.label = best.label;
    sug.category = "benchmark";
    sug.priority = "low";
    sug.text = best.label + " is the campus benchmark right now at " +
               human_num(best.co2_per_occupant, 2) + " kg CO2 per person this week.";
    out.push_back(sug);
  }

  auto prio_rank = [](const std::string& p) {
    if (p == "high") return 0;
    if (p == "medium") return 1;
    return 2;
  };
  std::stable_sort(out.begin(), out.end(), [&](const Suggestion& a, const Suggestion& b) {
    const int pa = prio_rank(a.priority);
    const int pb = prio_rank(b.priority);
    if (pa != pb) return pa < pb;
    return a.building < b.building;
  });
  if (out.size() > 8) out.resize(8);
  return out;
}

std::string json_suggestions(const std::vector<Suggestion>& suggestions) {
  std::ostringstream o;
  o << "{\"suggestions\":[";
  for (std::size_t i = 0; i < suggestions.size(); ++i) {
    const Suggestion& s = suggestions[i];
    if (i > 0) o << ",";
    o << "{\"building\":\"" << json_escape(s.building) << "\",\"label\":\"" << json_escape(s.label)
      << "\",\"category\":\"" << json_escape(s.category) << "\",\"priority\":\""
      << json_escape(s.priority) << "\",\"text\":\"" << json_escape(s.text) << "\"}";
  }
  o << "]}";
  return o.str();
}

}
