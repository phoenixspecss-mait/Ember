pub fn grouped(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn human_num(v: f64, decimals: usize) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let s = format!("{:.1$}", v, decimals);
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
        ("-", r)
    } else {
        ("", s.as_str())
    };
    match rest.split_once('.') {
        Some((int_part, frac)) => format!("{sign}{}.{frac}", grouped(int_part)),
        None => format!("{sign}{}", grouped(rest)),
    }
}

pub fn int_text(v: f64) -> String {
    format!("{:.0}", if v.is_finite() { v } else { 0.0 })
}

pub fn signed_num(v: f64, decimals: usize) -> String {
    let body = human_num(v.abs(), decimals);
    if v < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}
