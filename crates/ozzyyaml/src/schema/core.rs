//! YAML 1.2.2 Core schema scalar construction.

use crate::value::Value;

const NULL_TAG: &str = "tag:yaml.org,2002:null";
const BOOL_TAG: &str = "tag:yaml.org,2002:bool";
const INT_TAG: &str = "tag:yaml.org,2002:int";
const FLOAT_TAG: &str = "tag:yaml.org,2002:float";
const STR_TAG: &str = "tag:yaml.org,2002:str";

/// Resolve a plain (unquoted) scalar with Core schema rules.
pub(crate) fn resolve_plain(s: &str) -> Value {
    if let Some(v) = try_null(s) {
        return v;
    }
    if let Some(v) = try_bool(s) {
        return v;
    }
    if let Some(v) = try_int(s) {
        return v;
    }
    if let Some(v) = try_float(s) {
        return v;
    }
    Value::String(s.to_owned())
}

/// Resolve a scalar given its resolved tag URI (if any) and style.
///
/// Unknown non-Core tags are ignored for typing (value kept as string when
/// the style would otherwise stay a string; plain still runs Core resolution
/// only when untagged).
pub(crate) fn resolve_scalar(value: &str, tag: Option<&str>, plain: bool) -> Value {
    match tag {
        Some(NULL_TAG) => try_null(value).unwrap_or(Value::Null),
        Some(BOOL_TAG) => try_bool(value).unwrap_or_else(|| Value::String(value.to_owned())),
        Some(INT_TAG) => try_int(value).unwrap_or_else(|| Value::String(value.to_owned())),
        Some(FLOAT_TAG) => try_float(value).unwrap_or_else(|| Value::String(value.to_owned())),
        Some(STR_TAG) => Value::String(value.to_owned()),
        Some(_) => {
            // Unknown tag: ignore for typing.
            if plain {
                resolve_plain(value)
            } else {
                Value::String(value.to_owned())
            }
        }
        None if plain => resolve_plain(value),
        None => Value::String(value.to_owned()),
    }
}

fn try_null(s: &str) -> Option<Value> {
    match s {
        "" | "~" | "null" | "Null" | "NULL" => Some(Value::Null),
        _ => None,
    }
}

fn try_bool(s: &str) -> Option<Value> {
    match s {
        "true" | "True" | "TRUE" => Some(Value::Bool(true)),
        "false" | "False" | "FALSE" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn try_int(s: &str) -> Option<Value> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return parse_digits(hex, 16).map(Value::Int);
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return parse_digits(oct, 8).map(Value::Int);
    }
    // Decimal: optional sign + digits (no leading base prefix).
    let (sign, digits) = strip_sign(s)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Reject floats that look like they have a trailing dot handled elsewhere;
    // Core decimal int is [-+]?[0-9]+ only.
    let n = parse_digits(digits, 10)?;
    Some(Value::Int(if sign { -n } else { n }))
}

fn try_float(s: &str) -> Option<Value> {
    match s {
        ".nan" | ".NaN" | ".NAN" => return Some(Value::Float(f64::NAN)),
        _ => {}
    }
    if let Some(rest) = s.strip_prefix('+') {
        return match rest {
            ".inf" | ".Inf" | ".INF" => Some(Value::Float(f64::INFINITY)),
            _ => parse_finite_float(rest).map(Value::Float),
        };
    }
    if let Some(rest) = s.strip_prefix('-') {
        return match rest {
            ".inf" | ".Inf" | ".INF" => Some(Value::Float(f64::NEG_INFINITY)),
            _ => parse_finite_float(rest).map(|f| Value::Float(-f)),
        };
    }
    match s {
        ".inf" | ".Inf" | ".INF" => Some(Value::Float(f64::INFINITY)),
        _ => parse_finite_float(s).map(Value::Float),
    }
}

/// Core float (without leading sign): `(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`
fn parse_finite_float(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut i = 0usize;
    let mut saw_dot = false;

    if bytes[i] == b'.' {
        saw_dot = true;
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
    } else {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            saw_dot = true;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
    }

    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return None;
        }
    }

    if i != bytes.len() {
        return None;
    }
    // Integers without `.` or exponent are ints, not floats.
    if !saw_dot && !s.bytes().any(|b| b == b'e' || b == b'E') {
        return None;
    }
    s.parse().ok()
}

fn strip_sign(s: &str) -> Option<(bool, &str)> {
    if let Some(rest) = s.strip_prefix('+') {
        Some((false, rest))
    } else if let Some(rest) = s.strip_prefix('-') {
        Some((true, rest))
    } else {
        Some((false, s))
    }
}

fn parse_digits(digits: &str, radix: u32) -> Option<i64> {
    if digits.is_empty() {
        return None;
    }
    i64::from_str_radix(digits, radix).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_core_table() {
        let cases: &[(&str, Value)] = &[
            ("", Value::Null),
            ("~", Value::Null),
            ("null", Value::Null),
            ("NULL", Value::Null),
            ("true", Value::Bool(true)),
            ("FALSE", Value::Bool(false)),
            ("yes", Value::String("yes".into())),
            ("on", Value::String("on".into())),
            ("42", Value::Int(42)),
            ("-7", Value::Int(-7)),
            ("0o10", Value::Int(8)),
            ("0x1F", Value::Int(31)),
            ("1.5", Value::Float(1.5)),
            ("1e2", Value::Float(100.0)),
            (".inf", Value::Float(f64::INFINITY)),
            ("-.INF", Value::Float(f64::NEG_INFINITY)),
            ("hello", Value::String("hello".into())),
        ];
        for (input, expected) in cases {
            let got = resolve_plain(input);
            match (got, expected) {
                (Value::Float(a), Value::Float(b)) if a.is_nan() && b.is_nan() => {}
                (g, e) => assert_eq!(&g, e, "plain `{input}`"),
            }
        }
        assert!(matches!(resolve_plain(".nan"), Value::Float(f) if f.is_nan()));
    }

    #[test]
    fn tagged_str_stays_string() {
        assert_eq!(
            resolve_scalar("42", Some(STR_TAG), true),
            Value::String("42".into())
        );
    }

    #[test]
    fn quoted_untagged_stays_string() {
        assert_eq!(
            resolve_scalar("true", None, false),
            Value::String("true".into())
        );
    }
}
