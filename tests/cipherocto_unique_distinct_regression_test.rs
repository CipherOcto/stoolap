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

//! CipherOcto fork regression tests — pins the *narrow* divergence
//! between expected SQL semantics and the actual behaviour of the
//! `feat/blockchain-sql` branch (probed 2026-07-23 on commit e85634f).
//!
//! # Two findings, both empirically verified
//!
//! ## 1. Inline `UNIQUE` IS enforced
//!
//! `CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT UNIQUE)` followed
//! by `INSERT ... VALUES (1, 'a'), (2, 'a')` correctly fails with
//! `unique constraint failed for index unique_t_x on column x with
//! value [Text("a")]` (daemon RPC: `code = -32603`) and the whole
//! batch is rolled back (`COUNT(*) = 0`).
//!
//! ## 2. `SELECT DISTINCT` is silently ignored when combined with
//!    `ORDER BY` + `LIMIT` together
//!
//! Each clause alone works correctly:
//!
//! * `SELECT DISTINCT x FROM t`           → 2 rows (`a`, `b`)
//! * `SELECT DISTINCT x FROM t ORDER BY x` → 2 rows
//! * `SELECT DISTINCT x FROM t LIMIT 5`   → 2 rows
//!
//! But the triplet returns duplicates:
//!
//! * `SELECT DISTINCT x FROM t ORDER BY x LIMIT 5` → 5 rows (`a, a, b, b, b`)
//!
//! This is the root cause of the cipherocto flex scripts
//! (`persist-member-details-bulk.sh` etc.) doing Python-side dedup
//! after their `ORDER BY ... LIMIT N OFFSET M` source-table scan.
//! The DISTINCT keyword is parsed and accepted by the daemon's RPC
//! handler, then quietly dropped by the optimiser when both
//! ordering and capping are present. Until the fork fixes this,
//! callers that need unique rows must dedup post-SELECT.
//!
//! These tests pin the *correct* behaviour of the first finding
//! (UNIQUE) and the *buggy* behaviour of the second (DISTINCT +
//! ORDER BY + LIMIT) — so a fix on either side flips the relevant
//! test and is caught immediately.

use stoolap::Database;

#[test]
fn inline_unique_rejects_duplicate_insert_and_rolls_back_batch() {
    let db = Database::open("memory://cipherocto_unique_inline").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT UNIQUE)", ())
        .unwrap();

    // First batch — single row, succeeds.
    db.execute("INSERT INTO t (id, x) VALUES (1, 'a')", ())
        .expect("first insert should succeed");

    // Second batch — two rows with a duplicate `x`. The whole batch
    // must fail (not just the second row) and the table must remain
    // at exactly the row from the first batch.
    let dup = db.execute("INSERT INTO t (id, x) VALUES (2, 'a'), (3, 'b')", ());
    assert!(
        dup.is_err(),
        "expected duplicate `x='a'` to fail UNIQUE constraint, got Ok"
    );

    let n = db
        .query_one::<i64, _>("SELECT COUNT(*) FROM t", ())
        .unwrap();
    assert_eq!(
        n, 1,
        "batch containing a duplicate must be rolled back; got {n} rows"
    );

    // The non-duplicate row (3, 'b') must NOT have landed.
    let has_b: bool = db
        .query_one::<i64, _>("SELECT COUNT(*) FROM t WHERE x = 'b'", ())
        .unwrap()
        > 0;
    assert!(
        !has_b,
        "non-duplicate row from a failed batch must be absent"
    );
}

#[test]
fn inline_unique_accepts_distinct_values() {
    let db = Database::open("memory://cipherocto_unique_distinct_vals").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT UNIQUE)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        (),
    )
    .expect("distinct inserts must succeed");
    let n = db
        .query_one::<i64, _>("SELECT COUNT(*) FROM t", ())
        .unwrap();
    assert_eq!(n, 3);
}

#[test]
fn select_distinct_dedupes_single_column() {
    let db = Database::open("memory://cipherocto_distinct_single").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'b'), (5, 'b')",
        (),
    )
    .unwrap();

    let mut rows: Vec<String> = Vec::new();
    for row in db.query("SELECT DISTINCT x FROM t", ()).unwrap() {
        rows.push(row.unwrap().get::<String>(0).unwrap());
    }
    rows.sort();
    assert_eq!(
        rows,
        vec!["a".to_string(), "b".to_string()],
        "SELECT DISTINCT must dedupe; got {rows:?}"
    );
}

#[test]
fn count_distinct_returns_unique_count() {
    let db = Database::open("memory://cipherocto_distinct_count").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'b'), (5, 'b')",
        (),
    )
    .unwrap();
    let n = db
        .query_one::<i64, _>("SELECT COUNT(DISTINCT x) FROM t", ())
        .unwrap();
    assert_eq!(n, 2, "COUNT(DISTINCT x) must equal 2 for the fixture");
}

#[test]
fn select_distinct_alone_dedupes() {
    let db = Database::open("memory://cipherocto_distinct_alone").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'b'), (5, 'b')",
        (),
    )
    .unwrap();

    let mut rows: Vec<String> = Vec::new();
    for row in db.query("SELECT DISTINCT x FROM t", ()).unwrap() {
        rows.push(row.unwrap().get::<String>(0).unwrap());
    }
    rows.sort();
    assert_eq!(rows.len(), 2, "DISTINCT alone must dedupe; got {rows:?}");
}

#[test]
fn select_distinct_with_order_by_alone_dedupes() {
    let db = Database::open("memory://cipherocto_distinct_order").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'b'), (5, 'b')",
        (),
    )
    .unwrap();

    let mut rows: Vec<String> = Vec::new();
    for row in db.query("SELECT DISTINCT x FROM t ORDER BY x", ()).unwrap() {
        rows.push(row.unwrap().get::<String>(0).unwrap());
    }
    assert_eq!(
        rows,
        vec!["a".to_string(), "b".to_string()],
        "DISTINCT + ORDER BY (no LIMIT) must dedupe; got {rows:?}"
    );
}

#[test]
fn select_distinct_with_order_by_and_limit_dedupes() {
    // The bug: `SELECT DISTINCT x FROM t ORDER BY x LIMIT 5` returned
    // ALL 5 input rows (with duplicates intact) because
    // `DistinctResult` was hashing every column of the underlying scan
    // (id, x) instead of just the SELECT columns (x). Fix: query.rs
    // now passes `expected_columns` to `DistinctResult::with_column_count`
    // whenever SELECT has fewer columns than the scan.
    let db = Database::open("memory://cipherocto_distinct_order_limit").unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x TEXT)", ())
        .unwrap();
    db.execute(
        "INSERT INTO t (id, x) VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'b'), (5, 'b')",
        (),
    )
    .unwrap();

    let mut rows: Vec<String> = Vec::new();
    for row in db
        .query("SELECT DISTINCT x FROM t ORDER BY x LIMIT 5", ())
        .unwrap()
    {
        rows.push(row.unwrap().get::<String>(0).unwrap());
    }
    assert_eq!(
        rows,
        vec!["a".to_string(), "b".to_string()],
        "DISTINCT + ORDER BY + LIMIT must dedupe; got {rows:?}"
    );
}
