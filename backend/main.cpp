#include "httplib.h"

#include "analytics.hpp"
#include "seed.hpp"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <ctime>
#include <iostream>
#include <mutex>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace ember {
namespace json {

struct Value {
  enum class Type { Null, Bool, Number, String, Array, Object };
  Type type = Type::Null;
  bool boolean = false;
  double number = 0.0;
  std::string text;
  std::vector<Value> items;
  std::vector<std::pair<std::string, Value>> members;

  const Value* find(const std::string& key) const {
    for (const auto& kv : members) {
      if (kv.first == key) return &kv.second;
    }
    return nullptr;
  }
};

class Parser {
 public:
  explicit Parser(const std::string& src) : src_(src) {}

  Value parse() {
    skip_ws();
    Value v = parse_value(0);
    skip_ws();
    if (pos_ != src_.size()) fail("Unexpected trailing characters.");
    return v;
  }

 private:
  const std::string& src_;
  std::size_t pos_ = 0;

  static void fail(const char* msg) { throw std::runtime_error(msg); }

  void skip_ws() {
    while (pos_ < src_.size()) {
      const char c = src_[pos_];
      if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
        ++pos_;
      } else {
        break;
      }
    }
  }

  char peek() {
    if (pos_ >= src_.size()) fail("Unexpected end of JSON.");
    return src_[pos_];
  }

  void expect_word(const char* word) {
    for (const char* p = word; *p != '\0'; ++p) {
      if (pos_ >= src_.size() || src_[pos_] != *p) fail("Invalid JSON literal.");
      ++pos_;
    }
  }

  Value parse_value(int depth) {
    if (depth > 32) fail("JSON nesting too deep.");
    const char c = peek();
    if (c == '{') return parse_object(depth);
    if (c == '[') return parse_array(depth);
    if (c == '"') {
      Value v;
      v.type = Value::Type::String;
      v.text = parse_string();
      return v;
    }
    if (c == 't') {
      expect_word("true");
      Value v;
      v.type = Value::Type::Bool;
      v.boolean = true;
      return v;
    }
    if (c == 'f') {
      expect_word("false");
      Value v;
      v.type = Value::Type::Bool;
      return v;
    }
    if (c == 'n') {
      expect_word("null");
      return Value();
    }
    if (c == '-' || (c >= '0' && c <= '9')) return parse_number();
    fail("Invalid JSON value.");
  }

  Value parse_object(int depth) {
    Value v;
    v.type = Value::Type::Object;
    ++pos_;
    skip_ws();
    if (peek() == '}') {
      ++pos_;
      return v;
    }
    while (true) {
      skip_ws();
      if (peek() != '"') fail("Expected string key in object.");
      std::string key = parse_string();
      skip_ws();
      if (peek() != ':') fail("Expected ':' in object.");
      ++pos_;
      skip_ws();
      v.members.emplace_back(std::move(key), parse_value(depth + 1));
      skip_ws();
      const char c = peek();
      if (c == ',') {
        ++pos_;
        continue;
      }
      if (c == '}') {
        ++pos_;
        return v;
      }
      fail("Expected ',' or '}' in object.");
    }
  }

  Value parse_array(int depth) {
    Value v;
    v.type = Value::Type::Array;
    ++pos_;
    skip_ws();
    if (peek() == ']') {
      ++pos_;
      return v;
    }
    while (true) {
      skip_ws();
      v.items.push_back(parse_value(depth + 1));
      skip_ws();
      const char c = peek();
      if (c == ',') {
        ++pos_;
        continue;
      }
      if (c == ']') {
        ++pos_;
        return v;
      }
      fail("Expected ',' or ']' in array.");
    }
  }

  std::string parse_string() {
    ++pos_;
    std::string out;
    while (true) {
      if (pos_ >= src_.size()) fail("Unterminated string.");
      const char c = src_[pos_];
      if (c == '"') {
        ++pos_;
        return out;
      }
      if (c == '\\') {
        ++pos_;
        if (pos_ >= src_.size()) fail("Unterminated escape.");
        const char e = src_[pos_];
        switch (e) {
          case '"': out += '"'; ++pos_; break;
          case '\\': out += '\\'; ++pos_; break;
          case '/': out += '/'; ++pos_; break;
          case 'b': out += '\b'; ++pos_; break;
          case 'f': out += '\f'; ++pos_; break;
          case 'n': out += '\n'; ++pos_; break;
          case 'r': out += '\r'; ++pos_; break;
          case 't': out += '\t'; ++pos_; break;
          case 'u': {
            ++pos_;
            unsigned cp = parse_hex4();
            if (cp >= 0xD800u && cp <= 0xDBFFu) {
              if (pos_ + 1 < src_.size() && src_[pos_] == '\\' && src_[pos_ + 1] == 'u') {
                pos_ += 2;
                const unsigned lo = parse_hex4();
                if (lo >= 0xDC00u && lo <= 0xDFFFu) {
                  cp = 0x10000u + ((cp - 0xD800u) << 10) + (lo - 0xDC00u);
                } else {
                  fail("Invalid surrogate pair.");
                }
              } else {
                cp = 0xFFFDu;
              }
            } else if (cp >= 0xDC00u && cp <= 0xDFFFu) {
              cp = 0xFFFDu;
            }
            append_utf8(out, cp);
            break;
          }
          default: fail("Invalid escape sequence.");
        }
      } else {
        out += c;
        ++pos_;
      }
    }
  }

  unsigned parse_hex4() {
    if (pos_ + 4 > src_.size()) fail("Invalid unicode escape.");
    unsigned v = 0;
    for (int i = 0; i < 4; ++i) {
      const char c = src_[pos_ + static_cast<std::size_t>(i)];
      v <<= 4;
      if (c >= '0' && c <= '9') {
        v += static_cast<unsigned>(c - '0');
      } else if (c >= 'a' && c <= 'f') {
        v += static_cast<unsigned>(c - 'a' + 10);
      } else if (c >= 'A' && c <= 'F') {
        v += static_cast<unsigned>(c - 'A' + 10);
      } else {
        fail("Invalid unicode escape.");
      }
    }
    pos_ += 4;
    return v;
  }

  static void append_utf8(std::string& out, unsigned cp) {
    if (cp < 0x80u) {
      out += static_cast<char>(cp);
    } else if (cp < 0x800u) {
      out += static_cast<char>(0xC0u | (cp >> 6));
      out += static_cast<char>(0x80u | (cp & 0x3Fu));
    } else if (cp < 0x10000u) {
      out += static_cast<char>(0xE0u | (cp >> 12));
      out += static_cast<char>(0x80u | ((cp >> 6) & 0x3Fu));
      out += static_cast<char>(0x80u | (cp & 0x3Fu));
    } else {
      out += static_cast<char>(0xF0u | (cp >> 18));
      out += static_cast<char>(0x80u | ((cp >> 12) & 0x3Fu));
      out += static_cast<char>(0x80u | ((cp >> 6) & 0x3Fu));
      out += static_cast<char>(0x80u | (cp & 0x3Fu));
    }
  }

  Value parse_number() {
    const std::size_t start = pos_;
    if (peek() == '-') ++pos_;
    while (pos_ < src_.size()) {
      const char c = src_[pos_];
      if ((c >= '0' && c <= '9') || c == '+' || c == '-' || c == '.' || c == 'e' || c == 'E') {
        ++pos_;
      } else {
        break;
      }
    }
    if (pos_ == start) fail("Invalid number.");
    const std::string token = src_.substr(start, pos_ - start);
    char* endp = nullptr;
    const double v = std::strtod(token.c_str(), &endp);
    if (endp == nullptr || *endp != '\0') fail("Invalid number.");
    Value out;
    out.type = Value::Type::Number;
    out.number = v;
    return out;
  }
};

}  // namespace json
}  // namespace ember

namespace {

const char* kJsonType = "application/json; charset=utf-8";

std::string valid_ids() {
  std::string ids;
  for (const ember::Building& b : ember::buildings()) {
    if (!ids.empty()) ids += ", ";
    ids += b.id;
  }
  return ids;
}

std::string unknown_building_msg(const std::string& id) {
  return "Unknown building \"" + id + "\". Valid ids: " + valid_ids() + ".";
}

bool is_leap(int y) { return (y % 4 == 0 && y % 100 != 0) || y % 400 == 0; }

bool valid_date(const std::string& s, long* out_civil) {
  if (s.size() != 10 || s[4] != '-' || s[7] != '-') return false;
  for (std::size_t i = 0; i < s.size(); ++i) {
    if (i == 4 || i == 7) continue;
    if (s[i] < '0' || s[i] > '9') return false;
  }
  const int y = std::stoi(s.substr(0, 4));
  const int m = std::stoi(s.substr(5, 2));
  const int d = std::stoi(s.substr(8, 2));
  if (y < 2000 || y > 2100 || m < 1 || m > 12 || d < 1) return false;
  static const int mdays[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
  int dim = mdays[m - 1];
  if (m == 2 && is_leap(y)) dim = 29;
  if (d > dim) return false;
  *out_civil = ember::days_from_civil(y, static_cast<unsigned>(m), static_cast<unsigned>(d));
  return true;
}

}  // namespace

int main() {
  std::ios::sync_with_stdio(false);

  std::vector<ember::Reading> store = ember::seed_readings();
  std::mutex store_mtx;
  const auto started = std::chrono::steady_clock::now();
  std::cout << "ember seeded " << store.size() << " readings for " << ember::buildings().size()
            << " buildings" << std::endl;

  httplib::Server svr;

  svr.set_default_headers({
      {"Access-Control-Allow-Origin", "*"},
      {"Access-Control-Allow-Methods", "GET, POST, OPTIONS"},
      {"Access-Control-Allow-Headers", "Content-Type"},
      {"Access-Control-Max-Age", "86400"},
  });

  svr.Options(".*", [](const httplib::Request&, httplib::Response& res) { res.status = 204; });

  svr.set_error_handler([](const httplib::Request&, httplib::Response& res) {
    if (res.body.empty()) {
      res.status = res.status == 200 ? 404 : res.status;
      res.set_content("{\"error\":\"Not found.\"}", kJsonType);
    } else {
      res.set_content(res.body, kJsonType);
    }
  });

  svr.set_exception_handler(
      [](const httplib::Request&, httplib::Response& res, std::exception_ptr) {
        res.status = 500;
        res.set_content("{\"error\":\"Internal error.\"}", kJsonType);
      });

  svr.set_logger([](const httplib::Request& req, const httplib::Response& res) {
    std::cout << req.method << " " << req.path << " -> " << res.status << std::endl;
  });

  svr.Get("/health", [&](const httplib::Request&, httplib::Response& res) {
    std::size_t count = 0;
    {
      std::lock_guard<std::mutex> lock(store_mtx);
      count = store.size();
    }
    const long uptime = static_cast<long>(
        std::chrono::duration_cast<std::chrono::seconds>(std::chrono::steady_clock::now() - started)
            .count());
    res.set_content("{\"ok\":true,\"service\":\"ember\",\"buildings\":" +
                        std::to_string(ember::buildings().size()) +
                        ",\"readings\":" + std::to_string(count) +
                        ",\"uptime_s\":" + std::to_string(uptime) + "}",
                    kJsonType);
  });

  svr.Post("/readings", [&](const httplib::Request& req, httplib::Response& res) {
    auto bad = [&](const std::string& msg) {
      res.status = 400;
      res.set_content("{\"error\":\"" + ember::json_escape(msg) + "\"}", kJsonType);
    };

    ember::json::Value body;
    try {
      body = ember::json::Parser(req.body).parse();
    } catch (const std::exception&) {
      bad("Body must be valid JSON.");
      return;
    }
    if (body.type != ember::json::Value::Type::Object) {
      bad("Body must be a JSON object.");
      return;
    }

    const ember::json::Value* bfield = body.find("building");
    if (bfield == nullptr) {
      bad("Missing field: building");
      return;
    }
    if (bfield->type != ember::json::Value::Type::String) {
      bad("building must be a string.");
      return;
    }
    const ember::Building* b = ember::find_building(bfield->text);
    if (b == nullptr) {
      bad(unknown_building_msg(bfield->text));
      return;
    }

    const ember::json::Value* dfield = body.find("date");
    if (dfield == nullptr) {
      bad("Missing field: date");
      return;
    }
    if (dfield->type != ember::json::Value::Type::String) {
      bad("date must be a string.");
      return;
    }
    long civil = 0;
    if (!valid_date(dfield->text, &civil)) {
      bad("date must be a real calendar day formatted YYYY-MM-DD.");
      return;
    }
    if (civil > ember::today_civil() + 30) {
      bad("date cannot be more than 30 days in the future.");
      return;
    }

    std::string err;
    auto read_num = [&](const ember::json::Value* parent, const char* key, double cap,
                        double* out) -> bool {
      if (parent == nullptr) {
        err = std::string("Missing field: ") + key;
        return false;
      }
      const ember::json::Value* f = parent->find(key);
      if (f == nullptr) {
        err = std::string("Missing field: ") + key;
        return false;
      }
      if (f->type != ember::json::Value::Type::Number || !std::isfinite(f->number) ||
          f->number < 0.0) {
        err = std::string(key) + " must be a non-negative number.";
        return false;
      }
      if (f->number > cap) {
        err = std::string(key) + " must be at most " + ember::json_num(cap) + ".";
        return false;
      }
      *out = f->number;
      return true;
    };

    ember::Reading r;
    r.building = b->id;
    r.date = dfield->text;
    if (!read_num(&body, "kwh", 50000.0, &r.kwh)) {
      bad(err);
      return;
    }
    if (!read_num(&body, "water_l", 2000000.0, &r.water_l)) {
      bad(err);
      return;
    }

    const ember::json::Value* wf = body.find("waste_kg");
    if (wf == nullptr) {
      bad("Missing field: waste_kg");
      return;
    }
    if (wf->type != ember::json::Value::Type::Object) {
      bad("waste_kg must be an object.");
      return;
    }
    if (!read_num(wf, "organic", 20000.0, &r.waste.organic)) {
      bad(err);
      return;
    }
    if (!read_num(wf, "plastic", 20000.0, &r.waste.plastic)) {
      bad(err);
      return;
    }
    if (!read_num(wf, "paper", 20000.0, &r.waste.paper)) {
      bad(err);
      return;
    }
    if (!read_num(wf, "ewaste", 20000.0, &r.waste.ewaste)) {
      bad(err);
      return;
    }

    bool updated = false;
    {
      std::lock_guard<std::mutex> lock(store_mtx);
      for (ember::Reading& existing : store) {
        if (existing.building == r.building && existing.date == r.date) {
          existing = r;
          updated = true;
          break;
        }
      }
      if (!updated) {
        store.push_back(r);
        std::sort(store.begin(), store.end(),
                  [](const ember::Reading& a, const ember::Reading& c) {
                    if (a.building != c.building) return a.building < c.building;
                    return a.date < c.date;
                  });
      }
    }

    res.status = updated ? 200 : 201;
    res.set_content("{\"stored\":" + ember::json_reading(r) + ",\"updated\":" +
                        (updated ? "true" : "false") + "}",
                    kJsonType);
  });

  svr.Get("/readings", [&](const httplib::Request& req, httplib::Response& res) {
    auto bad = [&](const std::string& msg) {
      res.status = 400;
      res.set_content("{\"error\":\"" + ember::json_escape(msg) + "\"}", kJsonType);
    };

    const std::string building = req.get_param_value("building");
    if (!building.empty() && building != "all" && ember::find_building(building) == nullptr) {
      bad(unknown_building_msg(building));
      return;
    }

    int days = 60;
    const std::string days_s = req.get_param_value("days");
    if (!days_s.empty()) {
      char* endp = nullptr;
      const long v = std::strtol(days_s.c_str(), &endp, 10);
      if (endp == days_s.c_str() || *endp != '\0' || v < 1 || v > 180) {
        bad("days must be an integer between 1 and 180.");
        return;
      }
      days = static_cast<int>(v);
    }

    std::vector<ember::Reading> sel;
    {
      std::lock_guard<std::mutex> lock(store_mtx);
      sel = ember::select_readings(store, ember::today_civil(), building, days);
    }

    std::string out = "{\"readings\":[";
    for (std::size_t i = 0; i < sel.size(); ++i) {
      if (i > 0) out += ",";
      out += ember::json_reading(sel[i]);
    }
    out += "],\"count\":" + std::to_string(sel.size()) + "}";
    res.set_content(out, kJsonType);
  });

  auto snapshot = [&]() {
    std::lock_guard<std::mutex> lock(store_mtx);
    return store;
  };

  svr.Get("/summary", [&](const httplib::Request&, httplib::Response& res) {
    const std::vector<ember::Reading> copy = snapshot();
    res.set_content(ember::json_summary(ember::compute_summary(copy, ember::today_civil())),
                    kJsonType);
  });

  svr.Get("/alerts", [&](const httplib::Request&, httplib::Response& res) {
    const std::vector<ember::Reading> copy = snapshot();
    res.set_content(ember::json_alerts(ember::detect_alerts(copy, ember::today_civil())), kJsonType);
  });

  svr.Get("/leaderboard", [&](const httplib::Request&, httplib::Response& res) {
    const std::vector<ember::Reading> copy = snapshot();
    res.set_content(ember::json_leaderboard(ember::compute_leaderboard(copy, ember::today_civil())),
                    kJsonType);
  });

  svr.Get("/suggestions", [&](const httplib::Request&, httplib::Response& res) {
    const std::vector<ember::Reading> copy = snapshot();
    res.set_content(ember::json_suggestions(ember::build_suggestions(copy, ember::today_civil())),
                    kJsonType);
  });

  const char* port_env = std::getenv("PORT");
  int port = port_env != nullptr ? std::atoi(port_env) : 10000;
  if (port <= 0 || port > 65535) port = 10000;

  std::cout << "ember listening on 0.0.0.0:" << port << std::endl;
  if (!svr.listen("0.0.0.0", port)) {
    std::cout << "ember failed to bind 0.0.0.0:" << port << std::endl;
    return 1;
  }
  return 0;
}
