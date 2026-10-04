//! Instants on the wire: an RFC 3339 timestamp, or a plain `YYYY-MM-DD` date.
//!
//! Both appear in real documents. A timestamp is what a service emits; a date is what a
//! person writes, and the hand-written cases are real — a `resource.yaml` manifest, a
//! fixture, a query typed at a prompt. Refusing a date would make the format worse at
//! precisely the thing people do by hand.
//!
//! One module rather than an attribute per field, because the alternative was the bug
//! this file was extracted to fix: the query AST accepted a bare date and the descriptor
//! types did not, so whether `2026-01-01` parsed depended on which struct the field sat
//! in. Every instant in this crate goes through here, and the dialect is stated once.
//!
//! A date is read as midnight UTC and *written back* as a timestamp, so a round trip is a
//! fixed point rather than a reproduction of the caller's typing. Sub-second digits are
//! preserved when they are there and omitted when they are not: an instant recorded to
//! the millisecond is a different instant from the second it falls in, and rounding it on
//! the way through would make two results tie that did not.
//!
//! No mirrored test file, unlike every other source file here: the module is
//! `pub(crate)`, so it has no surface of its own to test at. It is exercised through the
//! public types that use it, in `tests/lib.rs` and `tests/query.rs`.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};

/// What a caller is told when a string is neither accepted form.
const EXPECTED: &str = "neither an RFC 3339 timestamp nor a YYYY-MM-DD date";

/// Read either accepted form, or nothing.
fn read(text: &str) -> Option<DateTime<Utc>> {
    if let Ok(instant) = DateTime::parse_from_rfc3339(text) {
        return Some(instant.with_timezone(&Utc));
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)
        .map(|naive| naive.and_utc())
}

/// Render an instant in the one form this crate writes.
fn write(instant: DateTime<Utc>) -> String {
    // `AutoSi` rather than `Secs`: it emits the sub-second digits that are there and none
    // that are not, so a whole second stays `…:00Z` and a millisecond stays a millisecond.
    instant.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// An instant a document may leave out.
pub(crate) mod optional {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    // `&Option<T>` rather than the `Option<&T>` clippy prefers: serde hands `with` a
    // reference to the field, so the signature belongs to serde and not to this module.
    // Rendering to `Option<String>` and letting serde serialize that keeps the `None`
    // case in serde's own impl, where it is already written and already correct — and in
    // practice it never runs, because every field using this is `skip_serializing_if`.
    #[allow(clippy::ref_option)]
    pub(crate) fn serialize<S>(
        value: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.map(super::write).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // An explicit `null` is the absence it looks like. A generator that emits every
        // key and fills the ones it has nothing for with `null` is common enough that
        // refusing it would be pedantry.
        let Some(text) = Option::<String>::deserialize(deserializer)? else {
            return Ok(None);
        };
        super::read(&text).map(Some).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "`{text}` is {expected}",
                expected = super::EXPECTED
            ))
        })
    }
}

/// An instant a document must carry.
pub(crate) mod required {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&super::write(*value))
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        super::read(&text).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "`{text}` is {expected}",
                expected = super::EXPECTED
            ))
        })
    }
}
