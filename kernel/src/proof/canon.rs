//! Canonical JSON bytes: the single serialization-for-hashing implementation.
//!
//! **Exactly one place** produces canonical JSON bytes in the kernel (SPINE-001
//! invariant, S1-M1-ONE-CANONICALIZER). All hashing flows that involve JSON
//! must route through this module.
//!
//! # Canonicalization rules
//!
//! 1. Object keys are sorted lexicographically (byte order).
//! 2. No extraneous whitespace (compact form: `{"a":1,"b":2}`).
//! 3. Strings are JSON-escaped per RFC 8259 §7.
//! 4. Numbers must be integers (`i64` or `u64`). Non-integer numbers (floats,
//!    NaN, Infinity) are rejected to prevent cross-platform formatting drift.
//! 5. `null`, `true`, `false` are written literally.
//! 6. Output is always valid UTF-8.

use std::io::Write;

/// Error type for canonical JSON serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonError {
    /// A JSON number was not an integer (float, NaN, Infinity).
    NonIntegerNumber { raw: String },
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonIntegerNumber { raw } => {
                write!(f, "non-integer number in canonical JSON: {raw}")
            }
        }
    }
}

/// Produce canonical JSON bytes from a `serde_json::Value`.
///
/// This is the single canonical JSON implementation in the kernel.
/// All hashing/digest flows that involve JSON must use this function.
///
/// # Errors
///
/// Returns [`CanonError::NonIntegerNumber`] if any JSON number is not
/// representable as `i64` or `u64` (floats, NaN, Infinity are rejected).
pub fn canonical_json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, CanonError> {
    let mut buf = Vec::new();
    write_value(&mut buf, value)?;
    Ok(buf)
}

fn write_value(buf: &mut Vec<u8>, value: &serde_json::Value) -> Result<(), CanonError> {
    match value {
        serde_json::Value::Null => {
            buf.extend_from_slice(b"null");
        }
        serde_json::Value::Bool(b) => {
            if *b {
                buf.extend_from_slice(b"true");
            } else {
                buf.extend_from_slice(b"false");
            }
        }
        serde_json::Value::Number(n) => {
            write_number(buf, n)?;
        }
        serde_json::Value::String(s) => {
            write_string(buf, s);
        }
        serde_json::Value::Array(arr) => {
            buf.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_value(buf, item)?;
            }
            buf.push(b']');
        }
        serde_json::Value::Object(map) => {
            // Sorted keys (lexicographic byte order).
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            buf.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_string(buf, key);
                buf.push(b':');
                write_value(buf, &map[*key])?;
            }
            buf.push(b'}');
        }
    }
    Ok(())
}

fn write_number(buf: &mut Vec<u8>, n: &serde_json::Number) -> Result<(), CanonError> {
    // Try i64 first (handles negatives), then u64 (handles large positives).
    if let Some(i) = n.as_i64() {
        let _ = write!(buf, "{i}");
        Ok(())
    } else if let Some(u) = n.as_u64() {
        let _ = write!(buf, "{u}");
        Ok(())
    } else {
        // Float, NaN, Infinity — reject.
        Err(CanonError::NonIntegerNumber {
            raw: n.to_string(),
        })
    }
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    buf.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => buf.extend_from_slice(b"\\\""),
            '\\' => buf.extend_from_slice(b"\\\\"),
            '\n' => buf.extend_from_slice(b"\\n"),
            '\r' => buf.extend_from_slice(b"\\r"),
            '\t' => buf.extend_from_slice(b"\\t"),
            // Control characters U+0000..U+001F (except those handled above).
            c if c < '\u{0020}' => {
                let _ = write!(buf, "\\u{:04x}", c as u32);
            }
            c => {
                let mut utf8_buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut utf8_buf);
                buf.extend_from_slice(encoded.as_bytes());
            }
        }
    }
    buf.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorted_keys() {
        let v = json!({"z": 1, "a": 2, "m": 3});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":2,\"m\":3,\"z\":1}");
    }

    #[test]
    fn nested_sorted_keys() {
        let v = json!({"b": {"d": 1, "c": 2}, "a": 3});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":3,\"b\":{\"c\":2,\"d\":1}}");
    }

    #[test]
    fn compact_no_whitespace() {
        let v: serde_json::Value =
            serde_json::from_str("{ \"a\" : 1 , \"b\" : [ 2 , 3 ] }").unwrap();
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":1,\"b\":[2,3]}");
    }

    #[test]
    fn ordering_invariance() {
        // Same logical object, different key insertion order.
        let v1: serde_json::Value = serde_json::from_str(r#"{"x":1,"a":2,"m":3}"#).unwrap();
        let v2: serde_json::Value = serde_json::from_str(r#"{"a":2,"m":3,"x":1}"#).unwrap();
        let v3: serde_json::Value = serde_json::from_str(r#"{"m":3,"x":1,"a":2}"#).unwrap();
        let b1 = canonical_json_bytes(&v1).unwrap();
        let b2 = canonical_json_bytes(&v2).unwrap();
        let b3 = canonical_json_bytes(&v3).unwrap();
        assert_eq!(b1, b2);
        assert_eq!(b2, b3);
    }

    #[test]
    fn whitespace_invariance() {
        let compact: serde_json::Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        let spaced: serde_json::Value = serde_json::from_str("{ \"a\" : 1 }").unwrap();
        let newlined: serde_json::Value =
            serde_json::from_str("{\n  \"a\": 1\n}").unwrap();
        let b1 = canonical_json_bytes(&compact).unwrap();
        let b2 = canonical_json_bytes(&spaced).unwrap();
        let b3 = canonical_json_bytes(&newlined).unwrap();
        assert_eq!(b1, b2);
        assert_eq!(b2, b3);
    }

    #[test]
    fn rejects_float() {
        let v = json!({"a": 1.5});
        let err = canonical_json_bytes(&v).unwrap_err();
        assert!(matches!(err, CanonError::NonIntegerNumber { .. }));
    }

    #[test]
    fn accepts_integer_zero() {
        let v = json!({"a": 0});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":0}");
    }

    #[test]
    fn accepts_negative_integer() {
        let v = json!({"a": -42});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":-42}");
    }

    #[test]
    fn accepts_large_u64() {
        let v = json!({"a": u64::MAX});
        let bytes = canonical_json_bytes(&v).unwrap();
        let expected = format!("{{\"a\":{}}}", u64::MAX);
        assert_eq!(bytes, expected.as_bytes());
    }

    #[test]
    fn null_true_false() {
        let v = json!({"a": null, "b": true, "c": false});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":null,\"b\":true,\"c\":false}");
    }

    #[test]
    fn string_escaping() {
        let v = json!({"a": "line1\nline2\ttab\\slash\"quote"});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(
            bytes,
            b"{\"a\":\"line1\\nline2\\ttab\\\\slash\\\"quote\"}"
        );
    }

    #[test]
    fn control_char_escaping() {
        // U+0001 should be escaped as \u0001
        let v = json!({"a": "\u{0001}"});
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"{\"a\":\"\\u0001\"}");
    }

    #[test]
    fn empty_object_and_array() {
        assert_eq!(canonical_json_bytes(&json!({})).unwrap(), b"{}");
        assert_eq!(canonical_json_bytes(&json!([])).unwrap(), b"[]");
    }

    #[test]
    fn array_ordering_preserved() {
        let v = json!([3, 1, 2]);
        let bytes = canonical_json_bytes(&v).unwrap();
        assert_eq!(bytes, b"[3,1,2]");
    }

    #[test]
    fn deterministic_repeated_calls() {
        let v = json!({"z": [1, 2], "a": {"c": 3, "b": 4}});
        let first = canonical_json_bytes(&v).unwrap();
        for _ in 0..10 {
            assert_eq!(canonical_json_bytes(&v).unwrap(), first);
        }
    }

    #[test]
    fn unicode_passthrough() {
        let v = json!({"emoji": "hello 🌍"});
        let bytes = canonical_json_bytes(&v).unwrap();
        // UTF-8 bytes should pass through, not be \u-escaped.
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            r#"{"emoji":"hello 🌍"}"#
        );
    }
}
