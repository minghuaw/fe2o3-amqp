//! Human-readable [`Display`] implementation for [`Value`].
//!
//! Unlike the derived [`Debug`], this drops the AMQP type tags and renders the
//! payload the way a human reads it: bare numbers, quoted strings, hyphenated
//! UUIDs, hex binary, ISO-8601 timestamps, and recursively formatted lists,
//! maps, arrays, and described values. The output is intended for logs and
//! diagnostics, not for round-tripping back into a `Value`.

use std::fmt::{self, Display, Formatter, Write};

use crate::descriptor::Descriptor;

use super::Value;

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Described(described) => {
                write_descriptor(f, &described.descriptor)?;
                f.write_str(" -> ")?;
                Display::fmt(&described.value, f)
            }
            Value::Null => f.write_str("null"),
            Value::Bool(v) => Display::fmt(v, f),
            Value::Ubyte(v) => Display::fmt(v, f),
            Value::Ushort(v) => Display::fmt(v, f),
            Value::Uint(v) => Display::fmt(v, f),
            Value::Ulong(v) => Display::fmt(v, f),
            Value::Byte(v) => Display::fmt(v, f),
            Value::Short(v) => Display::fmt(v, f),
            Value::Int(v) => Display::fmt(v, f),
            Value::Long(v) => Display::fmt(v, f),
            Value::Float(v) => Display::fmt(&v.0, f),
            Value::Double(v) => Display::fmt(&v.0, f),
            // Decimals hold IEEE-754 BID-encoded bytes with no lossless decimal
            // text form, so render the raw bytes as hex like `Binary`.
            Value::Decimal32(v) => write_hex_bytes(f, v.as_inner()),
            Value::Decimal64(v) => write_hex_bytes(f, v.as_inner()),
            Value::Decimal128(v) => write_hex_bytes(f, v.as_inner()),
            // `{:?}` renders a `char` with single quotes (e.g. `'a'`), which keeps
            // it visually distinct from a single-character `String`.
            Value::Char(v) => write!(f, "{:?}", v),
            Value::Timestamp(v) => write_timestamp(f, v.milliseconds()),
            Value::Uuid(v) => write_uuid(f, v.as_inner()),
            Value::Binary(v) => write_hex_bytes(f, v),
            // `{:?}` quotes and escapes the string (e.g. `"hello"`).
            Value::String(v) => write!(f, "{:?}", v),
            Value::Symbol(v) => write!(f, ":{}", v.0),
            Value::List(v) => write_seq(f, v),
            Value::Map(v) => {
                f.write_char('{')?;
                for (i, (key, val)) in v.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    Display::fmt(key, f)?;
                    f.write_str(": ")?;
                    Display::fmt(val, f)?;
                }
                f.write_char('}')
            }
            Value::Array(v) => write_seq(f, &v.0),
        }
    }
}

/// Render a descriptor as `:name` for a symbolic descriptor or `0x..` for a
/// numeric code, mirroring how a `Symbol` value is displayed.
fn write_descriptor(f: &mut Formatter<'_>, descriptor: &Descriptor) -> fmt::Result {
    match descriptor {
        Descriptor::Name(symbol) => write!(f, ":{}", symbol.0),
        Descriptor::Code(code) => write!(f, "0x{:x}", code),
    }
}

/// Render a slice of values as `[a, b, c]`.
fn write_seq(f: &mut Formatter<'_>, values: &[Value]) -> fmt::Result {
    f.write_char('[')?;
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        Display::fmt(value, f)?;
    }
    f.write_char(']')
}

/// Render raw bytes as `0x[deadbeef]`.
fn write_hex_bytes(f: &mut Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    f.write_str("0x[")?;
    for byte in bytes {
        write!(f, "{:02x}", byte)?;
    }
    f.write_char(']')
}

/// Render a 16-byte UUID in the canonical `8-4-4-4-12` hyphenated hex form.
fn write_uuid(f: &mut Formatter<'_>, bytes: &[u8; 16]) -> fmt::Result {
    for (i, byte) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            f.write_char('-')?;
        }
        write!(f, "{:02x}", byte)?;
    }
    Ok(())
}

/// Render an AMQP timestamp (`i64` milliseconds since the Unix epoch) as an
/// ISO-8601 / RFC-3339 UTC datetime with millisecond precision, e.g.
/// `2024-06-01T00:00:00.000Z`.
///
/// The rendering is lossless across the entire `i64` range and does not depend
/// on any datetime crate: the millisecond fraction is always shown, and years
/// outside `0000..=9999` use the ISO-8601 expanded form with an explicit sign
/// and at least six digits (e.g. `+271821-04-20T00:00:00.000Z`).
//
// AIDEV-NOTE: timestamp formatting is intentionally hand-rolled (see
// `civil_from_days`) rather than routed through the optional `time`/`chrono`
// features, so the output is lossless across the full i64 range and identical
// regardless of which features are enabled. Verified against Python's
// `datetime` over 200k random in-range values. Don't replace it with a
// feature-gated datetime conversion without preserving both properties.
fn write_timestamp(f: &mut Formatter<'_>, millis: i64) -> fmt::Result {
    const MS_PER_DAY: i64 = 86_400_000;

    // Euclidean division so negative timestamps floor correctly: -1 ms is the
    // last millisecond of 1969-12-31, not a negative time-of-day.
    let days = millis.div_euclid(MS_PER_DAY);
    let ms_of_day = millis.rem_euclid(MS_PER_DAY);

    let hour = ms_of_day / 3_600_000;
    let minute = (ms_of_day / 60_000) % 60;
    let second = (ms_of_day / 1_000) % 60;
    let milli = ms_of_day % 1_000;

    let (year, month, day) = civil_from_days(days);

    if (0..=9999).contains(&year) {
        write!(f, "{:04}", year)?;
    } else {
        // ISO-8601 expanded year: explicit sign, at least six digits
        // (`{:+07}` is sign plus six, and widens for larger years).
        write!(f, "{:+07}", year)?;
    }
    write!(
        f,
        "-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        month, day, hour, minute, second, milli
    )
}

/// Convert a count of days since the Unix epoch (1970-01-01) into a proleptic
/// Gregorian `(year, month, day)`. Uses astronomical year numbering (year 0
/// exists), matching ISO-8601, and is exact across the full `i64` range.
///
/// This is Howard Hinnant's `civil_from_days` algorithm; see
/// <http://howardhinnant.github.io/date_algorithms.html#civil_from_days>.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so leap days fall at the end of the cycle.
    let z = days + 719_468;
    // `era` is the 400-year cycle; the offset keeps the division flooring for
    // negative `z` (plain integer division in Rust truncates toward zero).
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097; // day-of-era, [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day-of-year (Mar-based), [0, 365]
    let mp = (5 * doy + 2) / 153; // month, Mar=0..Feb=11
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = y + if month <= 2 { 1 } else { 0 };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use serde_bytes::ByteBuf;

    use crate::{
        described::Described,
        descriptor::Descriptor,
        primitives::{Array, Dec32, OrderedMap, Symbol, Timestamp, Uuid},
        Value,
    };

    #[test]
    fn display_scalars() {
        assert_eq!(Value::Null.to_string(), "null");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Long(-7).to_string(), "-7");
        assert_eq!(Value::Double(OrderedFloat(1.5)).to_string(), "1.5");
        assert_eq!(Value::Char('a').to_string(), "'a'");
    }

    #[test]
    fn display_timestamp_is_lossless_iso8601() {
        let cases = [
            (0_i64, "1970-01-01T00:00:00.000Z"),
            (1000, "1970-01-01T00:00:01.000Z"),
            // 2024-06-01T00:00:00Z, with a millisecond fraction preserved
            (1_717_200_000_123, "2024-06-01T00:00:00.123Z"),
            // negative timestamps floor into the previous day
            (-1, "1969-12-31T23:59:59.999Z"),
            (-1000, "1969-12-31T23:59:59.000Z"),
            // years beyond 9999 use the ISO-8601 expanded form
            (253_402_300_800_000, "+010000-01-01T00:00:00.000Z"),
        ];
        for (millis, expected) in cases {
            assert_eq!(
                Value::Timestamp(Timestamp::from_milliseconds(millis)).to_string(),
                expected,
                "millis = {millis}"
            );
        }
    }

    #[test]
    fn display_strings_and_symbols() {
        assert_eq!(Value::String("hello".to_string()).to_string(), "\"hello\"");
        // strings are escaped so the output is unambiguous
        assert_eq!(Value::String("a\"b".to_string()).to_string(), "\"a\\\"b\"");
        assert_eq!(
            Value::Symbol(Symbol::from("my-symbol")).to_string(),
            ":my-symbol"
        );
    }

    #[test]
    fn display_binary_decimal_uuid() {
        assert_eq!(
            Value::Binary(ByteBuf::from(vec![0xde, 0xad, 0xbe, 0xef])).to_string(),
            "0x[deadbeef]"
        );
        assert_eq!(
            Value::Decimal32(Dec32::from([0x01, 0x02, 0x03, 0x04])).to_string(),
            "0x[01020304]"
        );
        let uuid = Uuid::from([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ]);
        assert_eq!(
            Value::Uuid(uuid).to_string(),
            "00112233-4455-6677-8899-aabbccddeeff"
        );
    }

    #[test]
    fn display_compound() {
        let list = Value::List(vec![
            Value::Int(1),
            Value::String("hello".to_string()),
            Value::Null,
        ]);
        assert_eq!(list.to_string(), "[1, \"hello\", null]");

        let array = Value::Array(Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
        assert_eq!(array.to_string(), "[1, 2, 3]");

        let mut map = OrderedMap::new();
        map.insert(Value::Symbol(Symbol::from("k")), Value::Int(1));
        map.insert(Value::Symbol(Symbol::from("x")), Value::Null);
        assert_eq!(Value::Map(map).to_string(), "{:k: 1, :x: null}");
    }

    #[test]
    fn display_described() {
        let code = Value::Described(Box::new(Described {
            descriptor: Descriptor::Code(0x83),
            value: Value::List(vec![Value::Int(1), Value::Null]),
        }));
        assert_eq!(code.to_string(), "0x83 -> [1, null]");

        let name = Value::Described(Box::new(Described {
            descriptor: Descriptor::Name(Symbol::from("example:type")),
            value: Value::Int(7),
        }));
        assert_eq!(name.to_string(), ":example:type -> 7");
    }
}
