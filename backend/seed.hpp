#pragma once

#include "analytics.hpp"

#include <algorithm>
#include <cmath>
#include <random>
#include <vector>

namespace ember {

inline double seed_round(double v) { return std::round(v * 10.0) / 10.0; }

inline std::vector<Reading> seed_readings() {
  std::vector<Reading> out;
  const auto& catalog = buildings();
  out.reserve(catalog.size() * 60);

  const long today = today_civil();
  std::mt19937 rng(20261104ul);
  std::uniform_real_distribution<double> uniform(0.0, 1.0);
  auto next_noise = [&]() {
    double u1 = std::max(uniform(rng), 1e-9);
    double u2 = uniform(rng);
    return std::sqrt(-2.0 * std::log(u1)) * std::cos(6.283185307179586 * u2);
  };

  for (int d = 0; d < 60; ++d) {
    const long civil = today - 59 + d;
    const std::string date = iso_date(civil);
    const int dow = static_cast<int>((civil + 4) % 7);
    const bool weekend = dow == 0 || dow == 6;

    for (const Building& b : catalog) {
      Reading r;
      r.building = b.id;
      r.date = date;

      double kwh = 0.0;
      double water = 0.0;
      double waste = 0.0;

      if (b.type == "hostel") {
        static const double kwh_bias[4] = {1.00, 1.16, 1.02, 0.94};
        const std::size_t idx = static_cast<std::size_t>(b.id.back() - 'a');
        const bool hot = uniform(rng) < 0.30;
        const bool laundry = (d % 7 == 2) || (d % 7 == 3);
        kwh = (300.0 + b.occupants * 0.55) * kwh_bias[idx] * (weekend ? 1.08 : 1.0) * (hot ? 1.17 : 1.0);
        water = b.occupants * 108.0 * (laundry ? 1.25 : 1.0);
        waste = b.occupants * 0.36;
        r.waste.organic = waste * 0.55;
        r.waste.plastic = waste * 0.20;
        r.waste.paper = waste * 0.235;
        r.waste.ewaste = waste * 0.004;
        kwh *= (1.0 + 0.05 * next_noise());
        water *= (1.0 + 0.05 * next_noise());
        waste *= (1.0 + 0.09 * next_noise());
        r.waste.organic = waste * 0.55;
        r.waste.plastic = waste * 0.20;
        r.waste.paper = waste * 0.235;
        r.waste.ewaste = waste * 0.004;
      } else if (b.type == "library") {
        kwh = weekend ? (dow == 0 ? 90.0 : 170.0) : 300.0;
        water = weekend ? (dow == 0 ? 1800.0 : 2600.0) : 6200.0;
        waste = weekend ? (dow == 0 ? 7.0 : 10.0) : 18.0;
        kwh *= (1.0 + 0.06 * next_noise());
        water *= (1.0 + 0.08 * next_noise());
        waste *= (1.0 + 0.10 * next_noise());
        r.waste.organic = waste * 0.38;
        r.waste.plastic = waste * 0.24;
        r.waste.paper = waste * 0.36;
        r.waste.ewaste = waste * 0.008;
      } else {
        kwh = weekend ? (dow == 0 ? 260.0 : 300.0) : 760.0;
        water = weekend ? 10500.0 : 24000.0;
        waste = weekend ? (dow == 0 ? 55.0 : 60.0) : 125.0;
        kwh *= (1.0 + 0.06 * next_noise());
        water *= (1.0 + 0.07 * next_noise());
        waste *= (1.0 + 0.10 * next_noise());
        r.waste.organic = waste * 0.42;
        r.waste.plastic = waste * 0.26;
        r.waste.paper = waste * 0.30;
        r.waste.ewaste = waste * 0.006;
      }

      if (uniform(rng) < 0.02) {
        r.waste.ewaste += 1.6;
      }

      if (b.id == "main-block" && d == 33) {
        kwh *= 1.72;
      }
      if (b.id == "hostel-c" && (d == 41 || d == 42)) {
        water *= 2.45;
      }
      if (b.id == "library" && d == 22) {
        r.waste.organic *= 1.95;
        r.waste.plastic *= 1.95;
        r.waste.paper *= 1.95;
        r.waste.ewaste *= 1.95;
        waste *= 1.95;
      }
      if (b.id == "hostel-a" && (d == 12 || d == 13)) {
        kwh *= 1.55;
      }
      if (b.id == "hostel-d" && d == 55) {
        water *= 1.85;
      }
      if (b.id == "main-block" && d == 57) {
        r.waste.plastic *= 3.2;
        waste = r.waste.organic + r.waste.plastic + r.waste.paper + r.waste.ewaste;
      }

      r.kwh = seed_round(std::max(0.0, kwh));
      r.water_l = seed_round(std::max(0.0, water));
      r.waste.organic = seed_round(std::max(0.0, r.waste.organic));
      r.waste.plastic = seed_round(std::max(0.0, r.waste.plastic));
      r.waste.paper = seed_round(std::max(0.0, r.waste.paper));
      r.waste.ewaste = seed_round(std::max(0.0, r.waste.ewaste));
      (void)waste;

      out.push_back(r);
    }
  }

  return out;
}

}
