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
//! canonical 16-byte BE `DqaEncoding` wire form (value | scale |
//! reserved[7]) defined in `octo_determin::Dqa::to_bytes()`.
//!
//! AC-6 cases:
//!  - `Dqa::new(900_000, 0)`  round-trip byte-exact
//!  - `Dqa::new(900, 5)`      scale=5 preserved
//!  - `Dqa::new(0, 0)`        zero edge
//!  - `Dqa::new(-1, 0)`       negative edge
//!  - `Dqa::new(i64::MAX, 0)` max value edge

use octo_determin::Dqa;
use stoolap::{params, Database};

const DSN: &str = "memory://test_dqa_driver";

fn fresh_db(label: &str) -> Database {
    let db = Database::open(DSN).unwrap();
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
