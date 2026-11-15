#pragma once

#include <string>
#include <vector>

namespace ember {

struct WasteSplit {
  double organic = 0.0;
  double plastic = 0.0;
  double paper = 0.0;
  double ewaste = 0.0;
  double total() const;
};

struct Reading {
  std::string building;
  std::string date;
  double kwh = 0.0;
  double water_l = 0.0;
  WasteSplit waste;
};

struct Building {
  std::string id;
  std::string label;
  std::string type;
  int occupants = 0;
};

const std::vector<Building>& buildings();
const Building* find_building(const std::string& id);
bool is_hostel(const std::string& id);

double co2_from_kwh(double kwh);

long today_civil();
long days_from_civil(int y, unsigned m, unsigned d);
std::string iso_date(long civil_day);

struct Summary {
  int window_days = 7;
  std::string window_end;
  double kwh = 0.0;
  double co2_kg = 0.0;
  double water_l = 0.0;
  double waste_kg = 0.0;
  WasteSplit waste_split;
  double change_pct_kwh = 0.0;
  double change_pct_co2 = 0.0;
  double change_pct_water = 0.0;
  double change_pct_waste = 0.0;
  int campus_score = 0;
};

Summary compute_summary(const std::vector<Reading>& all, long today);

struct Alert {
  std::string building;
  std::string label;
  std::string date;
  std::string metric;
  double value = 0.0;
  double mean = 0.0;
  double sigma = 0.0;
  double ratio_sigma = 0.0;
  std::string reason;
};

std::vector<Alert> detect_alerts(const std::vector<Reading>& all, long today);

struct LeaderRow {
  int rank = 0;
  std::string building;
  std::string label;
  int occupants = 0;
  double kwh = 0.0;
  double co2_kg = 0.0;
  double co2_per_occupant = 0.0;
  int score = 0;
};

struct Leaderboard {
  std::vector<LeaderRow> rows;
  std::string generated_at;
};

Leaderboard compute_leaderboard(const std::vector<Reading>& all, long today);

struct Suggestion {
  std::string building;
  std::string label;
  std::string category;
  std::string priority;
  std::string text;
};

std::vector<Suggestion> build_suggestions(const std::vector<Reading>& all, long today);

std::vector<Reading> select_readings(const std::vector<Reading>& all, long today,
                                     const std::string& building, int days);

std::string json_escape(const std::string& s);
std::string json_num(double v);
std::string json_reading(const Reading& r);
std::string json_summary(const Summary& s);
std::string json_alerts(const std::vector<Alert>& alerts);
std::string json_leaderboard(const Leaderboard& lb);
std::string json_suggestions(const std::vector<Suggestion>& suggestions);

}
