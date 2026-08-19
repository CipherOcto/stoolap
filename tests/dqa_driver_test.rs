// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Integration tests for the `octo_determin::Dqa` driver surface
//! (CipherOcto mission 0900-d2 / fork RFC dqa-driver).
//!
//! Verifies that `r.get::<Dqa>(idx)` + `tx.set::<Dqa>(idx, value)`
//! round-trip losslessly through Stoolap DQA(0) columns, with the
//! 17-byte wire form (1 tag + 16-byte `DqaEncoding` payload: value
//! | scale | reserved[7]) laid out as in
//! `octo_determin::DqaEncoding::from_dqa`.
//!
//! Note: `Value::quant` writes the raw `{value, scale}` pair WITHOUT
//! canonicalizing. The canonical `DqaEncoding::to_bytes()` strips
//! trailing zeros first. For non-canonical inputs (e.g. `Dqa::new(900, 5)`)
//! the two paths produce different bytes. The fork codec is internally
//! consistent (round-trip preserves `{value, scale}`) but cross-comparison
//! with the borsh path requires canonicalizing the input first. See RFC
//! §Wire Form for the full contract; reviewers flagged this as a
//! follow-on (RFC-0105 amendment to expose `Dqa::canonicalize`).
//!
//! AC-6 cases:
//!  - `Dqa::new(900_000, 0)`  round-trip struct-equivalent
//!  - `Dqa::new(900, 5)`      scale=5 preserved
//!  - `Dqa::new(0, 0)`        zero edge
//!  - `Dqa::new(-1, 0)`       negative edge
//!  - `Dqa::new(i64::MAX, 0)` max value edge
//!  - `Dqa::new(1, 18)`       scale boundary max-valid
//!  - `Dqa::new(1, 19)`       scale boundary over-max (rejected)
//!  - non-Quant `Value::Integer(42)` rejected by `FromValue for Dqa`
//!  - reserved-bytes 10..17 non-zero rejected by `Value::as_dqa`
//!  - payload length < 17 OR > 17 rejected by `Value::as_dqa`

use octo_determin::Dqa;
use stoolap::common::CompactArc;
use stoolap::{params, DataType, Database, FromValue, Value};

fn fresh_db(label: &str) -> Database {
    let db = Database::open_in_memory().unwrap();
    db.execute(&format!("CREATE TABLE dqa_t_{label} (v DQA(0))"), ())
        .unwrap();
    db
}

fn read_back(db: &Database, table: &str, written: Dqa) -> Dqa {
    db.execute(
        &format!("INSERT INTO {table} VALUES ($1)"),
        params![written],
    )
    .unwrap();
    let sql = format!("SELECT v FROM {table}");
    let rows = db.query(&sql, ()).unwrap();
    for row in rows {
        let row = row.unwrap();
        return row.get::<Dqa>(0).unwrap();
    }
    panic!("SELECT returned no rows");
}

#[test]
fn dqa_roundtrip_900_000_scale0() {
    let db = fresh_db("r1");
    let written = Dqa::new(900_000, 0).unwrap();
    let read = read_back(&db, "dqa_t_r1", written);
    assert_eq!(read.value, written.value);
    assert_eq!(read.scale, written.scale);
}

#[test]
fn dqa_roundtrip_900_scale5() {
    let db = fresh_db("r2");
    let written = Dqa::new(900, 5).unwrap();
    let read = read_back(&db, "dqa_t_r2", written);
    assert_eq!(read.value, 900);
    assert_eq!(read.scale, 5, "scale=5 must survive the round-trip");
}

#[test]
fn dqa_roundtrip_zero() {
    let db = fresh_db("r3");
    let written = Dqa::new(0, 0).unwrap();
    let read = read_back(&db, "dqa_t_r3", written);
    assert_eq!(read.value, 0);
    assert_eq!(read.scale, 0);
}

#[test]
fn dqa_roundtrip_negative() {
    let db = fresh_db("r4");
    let written = Dqa::new(-1, 0).unwrap();
    let read = read_back(&db, "dqa_t_r4", written);
    assert_eq!(read.value, -1);
    assert_eq!(read.scale, 0);
}

#[test]
fn dqa_roundtrip_max() {
    let db = fresh_db("r5");
    let written = Dqa::new(i64::MAX, 0).unwrap();
    let read = read_back(&db, "dqa_t_r5", written);
    assert_eq!(read.value, i64::MAX);
    assert_eq!(read.scale, 0);
}

/// AC-6 scale boundary: `MAX_SCALE = 18` is the largest accepted
/// scale. Round-trip must preserve the 18-bit fractional position.
#[test]
fn dqa_roundtrip_max_scale_18() {
    let db = fresh_db("r6");
    let written = Dqa::new(1, 18).unwrap();
    let read = read_back(&db, "dqa_t_r6", written);
    assert_eq!(read.value, 1);
    assert_eq!(read.scale, 18);
}

/// AC-6 scale boundary: `scale > 18` is rejected by `Dqa::new`
/// at construction time. Confirms the codec propagates the
/// construction error path.
#[test]
fn dqa_constructor_rejects_scale_19() {
    let result = Dqa::new(1, 19);
    assert!(
        result.is_err(),
        "Dqa::new(1, 19) must be rejected (MAX_SCALE = 18)"
    );
}

/// Falsify: `FromValue for Dqa` MUST return Err for non-Quant Value
/// variants. Without this, a future refactor that broadens the
/// decoder could silently mis-decode.
#[test]
fn from_value_rejects_non_quant_integer() {
    let v = Value::Integer(42);
    let result = Dqa::from_value(&v);
    assert!(
        result.is_err(),
        "Dqa::from_value must reject Value::Integer"
    );
}

#[test]
fn from_value_rejects_non_quant_null() {
    let v = Value::Null(DataType::Null);
    let result = Dqa::from_value(&v);
    assert!(result.is_err(), "Dqa::from_value must reject Value::Null");
}

/// Falsify: `Value::as_dqa` MUST reject payloads with non-zero
/// reserved bytes (matches `DqaEncoding::to_dqa` validation).
#[test]
fn as_dqa_rejects_non_zero_reserved_bytes() {
    let mut buf = Vec::with_capacity(17);
    buf.push(9); // DataType::Quant = 9
    buf.extend_from_slice(&0_i64.to_be_bytes()); // value = 0
    buf.push(0); // scale = 0
    buf.extend_from_slice(&[0xFFu8; 7]); // reserved bytes ALL non-zero
    let v = Value::Extension(CompactArc::from(buf));
    assert!(
        v.as_dqa().is_none(),
        "as_dqa must reject non-zero reserved bytes"
    );
}

#[test]
fn as_dqa_rejects_short_payload() {
    let mut buf = Vec::with_capacity(10);
    buf.push(9);
    buf.extend_from_slice(&0_i64.to_be_bytes());
    buf.push(0);
    // NO reserved bytes — payload is 10 bytes, too short
    let v = Value::Extension(CompactArc::from(buf));
    assert!(
        v.as_dqa().is_none(),
        "as_dqa must reject payload < 17 bytes"
    );
}

#[test]
fn as_dqa_rejects_long_payload() {
    let mut buf = Vec::with_capacity(24);
    buf.push(9);
    buf.extend_from_slice(&0_i64.to_be_bytes());
    buf.push(0);
    buf.extend_from_slice(&[0u8; 15]); // 14 bytes of trailing payload
    let v = Value::Extension(CompactArc::from(buf));
    assert!(
        v.as_dqa().is_none(),
        "as_dqa must reject payload > 17 bytes"
    );
}

/// Multi-row round-trip: codec must work consistently across N rows
/// in a single table (catches row-iteration regressions).
#[test]
fn dqa_roundtrip_multi_row() {
    let db = fresh_db("r10");
    let fixtures = [
        Dqa::new(1, 0).unwrap(),
        Dqa::new(42, 2).unwrap(),
        Dqa::new(i64::MAX, 0).unwrap(),
        Dqa::new(i64::MIN, 0).unwrap(),
        Dqa::new(0, 18).unwrap(),
    ];
    for f in &fixtures {
        db.execute("INSERT INTO dqa_t_r10 VALUES ($1)", params![*f])
            .unwrap();
    }
    let rows = db.query("SELECT v FROM dqa_t_r10", ()).unwrap();
    let collected: Vec<Dqa> = rows.map(|r| r.unwrap().get::<Dqa>(0).unwrap()).collect();
    assert_eq!(collected.len(), fixtures.len());
    for (got, want) in collected.iter().zip(fixtures.iter()) {
        assert_eq!(got.value, want.value);
        assert_eq!(got.scale, want.scale);
    }
}
