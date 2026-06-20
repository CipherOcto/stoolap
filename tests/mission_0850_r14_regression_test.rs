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

//! Regression tests for two bugs surfaced by Mission 0850 R14 review
//! (octo-adapter-whatsapp cargo crate, RFC-0850 §8.6/§9.4 WhatsApp
//! native media transport):
//!
//! 1. **UPSERT on COMPOSITE unique index** — `INSERT ... ON DUPLICATE KEY
//!    UPDATE` does not work when the conflicting unique index spans
//!    multiple columns (e.g. `UNIQUE (name, index_mac, device_id)`).
//!    The executor's `find_row_by_unique_index` helper
//!    (`src/executor/dml.rs:2328`) treats the `column` field of the
//!    `UniqueConstraint` error as a single column name; for a composite
//!    index, `column` is a comma-space-separated list of column names
//!    (e.g. `"name, index_mac, device_id"`), so the helper's lookup in
//!    `schema.column_index_map()` misses and returns `None`. The
//!    executor then falls through to `Error::UniqueConstraint { value:
//!    "unknown" }` instead of running the UPDATE branch.
//!
//! 2. **In-memory backend transaction snapshot for DELETE+INSERT** —
//!    when a second transaction does a `DELETE` followed by an `INSERT`
//!    that targets a row committed by a prior transaction, the DELETE
//!    in the second transaction does not see the prior transaction's
//!    committed row. The subsequent INSERT then raises `UniqueConstraint`.
//!    The `database` is the in-memory backend (the only backend that
//!    exists today; the file-backed backend was removed in earlier
//!    versions of Stoolap per its `AGENTS.md`). This is a snapshot
//!    isolation bug in the MVCC engine.
//!
//! These tests should FAIL on the current main branch and PASS once the
//! two bugs are fixed.

use stoolap::Database;

/// Setup the table used by the UPSERT-on-composite-unique-index test.
fn setup_composite_unique_table(db: &Database) {
    db.execute(
        "CREATE TABLE app_state_mutation_macs (
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            index_mac BLOB NOT NULL,
            value_mac BLOB NOT NULL,
            device_id INTEGER NOT NULL,
            UNIQUE (name, index_mac, device_id)
        )",
        (),
    )
    .expect("Failed to create app_state_mutation_macs table");
}

/// Reproduce bug #1: `INSERT ... ON DUPLICATE KEY UPDATE` with a
/// composite unique index. Insert an initial row, then try to upsert a
/// row with the same `(name, index_mac, device_id)` triple but a
/// different `value_mac`. On the current main branch this fails with
/// `UniqueConstraint { value: "unknown" }` because
/// `find_row_by_unique_index` cannot find the conflicting row by the
/// composite column name.
#[test]
fn test_upsert_on_composite_unique_index() {
    let db = Database::open("memory://upsert_composite").expect("Failed to create database");
    setup_composite_unique_table(&db);

    // Insert initial row.
    db.execute(
        "INSERT INTO app_state_mutation_macs
         (name, version, index_mac, value_mac, device_id)
         VALUES ($1, $2, $3, $4, $5)",
        ("critical_block", 1i64, vec![0u8; 32], vec![0xAAu8; 32], 1i64),
    )
    .expect("Failed to insert initial row");

    // Upsert with the same composite-unique triple but a new value_mac.
    // This should UPDATE the existing row, not raise UniqueConstraint.
    db.execute(
        "INSERT INTO app_state_mutation_macs
         (name, version, index_mac, value_mac, device_id)
         VALUES ($1, $2, $3, $4, $5)
         ON DUPLICATE KEY UPDATE
           version = $2,
           value_mac = $4",
        ("critical_block", 2i64, vec![0u8; 32], vec![0x55u8; 32], 1i64),
    )
    .expect("Failed to upsert row with same composite unique triple");

    // Verify the row was updated (not duplicated).
    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM app_state_mutation_macs", ())
        .expect("Failed to count");
    assert_eq!(count, 1, "Expected 1 row after upsert (no duplicate)");

    let version: i64 = db
        .query_one(
            "SELECT version FROM app_state_mutation_macs
             WHERE name = $1 AND device_id = $2",
            ("critical_block", 1i64),
        )
        .expect("Failed to query version");
    assert_eq!(version, 2, "version must be updated to 2");

    let value_mac: Vec<u8> = db
        .query_one(
            "SELECT value_mac FROM app_state_mutation_macs
             WHERE name = $1 AND device_id = $2",
            ("critical_block", 1i64),
        )
        .expect("Failed to query value_mac");
    assert_eq!(
        value_mac,
        vec![0x55u8; 32],
        "value_mac must be updated to 0x55..."
    );
}

/// Reproduce bug #1 in a more general form: a table with a composite
/// unique index on two columns, where the second column is the
/// conflicting one. This pins that the fix must search by ALL columns
/// of the unique index, not just the first one.
#[test]
fn test_upsert_on_composite_unique_index_second_column_conflict() {
    let db = Database::open("memory://upsert_composite2").expect("Failed to create database");
    db.execute(
        "CREATE TABLE t (
            a TEXT NOT NULL,
            b TEXT NOT NULL,
            v INTEGER NOT NULL,
            UNIQUE (a, b)
        )",
        (),
    )
    .expect("Failed to create t table");

    db.execute(
        "INSERT INTO t (a, b, v) VALUES ($1, $2, $3)",
        ("x", "k1", 1i64),
    )
    .expect("Failed to insert (x, k1, 1)");

    // Upsert (x, k1, 99) — same (a, b) as existing row, new v. Should
    // update the existing row, not raise UniqueConstraint. Note: the
    // conflict is on column `b`, not column `a` — this pins that the
    // fix must search by ALL columns of the composite index.
    db.execute(
        "INSERT INTO t (a, b, v) VALUES ($1, $2, $3)
         ON DUPLICATE KEY UPDATE v = $3",
        ("x", "k1", 99i64),
    )
    .expect("Failed to upsert (x, k1, 99)");

    let v: i64 = db
        .query_one("SELECT v FROM t WHERE a = $1 AND b = $2", ("x", "k1"))
        .expect("Failed to query v");
    assert_eq!(v, 99, "v must be updated to 99");

    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM t", ())
        .expect("Failed to count");
    assert_eq!(count, 1, "Expected 1 row after upsert (no duplicate)");
}

/// Reproduce bug #2: in-memory backend's transaction snapshot does not
/// see rows committed by a prior transaction when the current
/// transaction does a DELETE+INSERT in the same tx (the DELETE doesn't
/// see the prior tx's row, the INSERT then raises UniqueConstraint).
/// This is the exact pattern that `put_mutation_macs` in
/// `octo-adapter-whatsapp/src/store.rs` needs to wrap atomically.
#[test]
fn test_tx_delete_insert_sees_prior_tx_committed_row() {
    let db = Database::open("memory://tx_delete_insert").expect("Failed to create database");
    db.execute(
        "CREATE TABLE app_state_mutation_macs (
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            index_mac BLOB NOT NULL,
            value_mac BLOB NOT NULL,
            device_id INTEGER NOT NULL,
            UNIQUE (name, index_mac, device_id)
        )",
        (),
    )
    .expect("Failed to create app_state_mutation_macs table");

    // First transaction: commit a row.
    let mut tx1 = db.begin().expect("Failed to begin tx1");
    tx1.execute(
        "INSERT INTO app_state_mutation_macs
         (name, version, index_mac, value_mac, device_id)
         VALUES ($1, $2, $3, $4, $5)",
        ("critical_block", 1i64, vec![0u8; 32], vec![0xAAu8; 32], 1i64),
    )
    .expect("Failed to insert in tx1");
    tx1.commit().expect("Failed to commit tx1");

    // Second transaction: DELETE the row (should see the row committed
    // by tx1) and INSERT a new one with the same composite-unique
    // triple but a different value_mac. This is the exact pattern that
    // octo-adapter-whatsapp's `put_mutation_macs` uses for per-mutation
    // idempotency.
    let mut tx2 = db.begin().expect("Failed to begin tx2");
    tx2.execute(
        "DELETE FROM app_state_mutation_macs
         WHERE name = $1 AND index_mac = $2 AND device_id = $3",
        ("critical_block", vec![0u8; 32], 1i64),
    )
    .expect("Failed to delete in tx2 (row from tx1 should be visible)");

    tx2.execute(
        "INSERT INTO app_state_mutation_macs
         (name, version, index_mac, value_mac, device_id)
         VALUES ($1, $2, $3, $4, $5)",
        ("critical_block", 2i64, vec![0u8; 32], vec![0x55u8; 32], 1i64),
    )
    .expect("Failed to insert in tx2 (after delete, should not violate unique constraint)");

    tx2.commit().expect("Failed to commit tx2");

    // Verify the new row is present with the new value_mac.
    let value_mac: Vec<u8> = db
        .query_one(
            "SELECT value_mac FROM app_state_mutation_macs
             WHERE name = $1 AND device_id = $2",
            ("critical_block", 1i64),
        )
        .expect("Failed to query value_mac");
    assert_eq!(
        value_mac,
        vec![0x55u8; 32],
        "value_mac must be the new one (0x55...) after tx2"
    );

    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM app_state_mutation_macs", ())
        .expect("Failed to count");
    assert_eq!(count, 1, "Expected exactly 1 row after tx2");
}

// ─── R15 regression test ────────────────────────────────────────────
//
// Surface: octo-adapter-whatsapp's `put_mutation_macs` (R15 fix):
// replaced the per-iteration DELETE+INSERT with a single
// `INSERT ... ON DUPLICATE KEY UPDATE` statement wrapped in a
// single transaction. Stoolap's `Database::execute` (the path used
// for top-level statements) had the UPSERT logic working, but the
// `Transaction::execute` API had a fast-path INSERT handler that
// called `table.insert()` directly and bypassed the executor's
// UPSERT code. So the cipherocto test
// `put_mutation_macs_is_idempotent_on_overwrite` would fail on the
// second call (within a transaction) because the conflict raised
// `UniqueConstraint` instead of triggering the UPDATE branch.
//
// This regression test pins the fix at the stoolap API level:
// UPSERT must work inside `Transaction::execute` against a
// COMPOSITE unique index (the same schema as the cipherocto
// adapter).
#[test]
fn test_upsert_in_api_transaction_works_on_composite_unique() {
    let db = Database::open("memory://r15_upsert_in_api_tx").expect("open db");
    db.execute(
        "CREATE TABLE app_state_mutation_macs (
            name TEXT NOT NULL,
            version BIGINT NOT NULL,
            index_mac BLOB NOT NULL,
            value_mac BLOB NOT NULL,
            device_id BIGINT NOT NULL,
            UNIQUE (name, index_mac, device_id)
        )",
        (),
    )
    .expect("create table");

    // First transaction: plain INSERT, commit.
    {
        let mut tx = db.begin().expect("begin tx1");
        tx.execute(
            "INSERT INTO app_state_mutation_macs
                (name, version, index_mac, value_mac, device_id)
             VALUES ($1, $2, $3, $4, $5)",
            (
                "critical_block",
                1i64,
                stoolap::core::Value::blob(vec![0u8; 32]),
                stoolap::core::Value::blob(vec![0x11u8; 32]),
                42i64,
            ),
        )
        .expect("first insert in tx1");
        tx.commit().expect("commit tx1");
    }

    // Second transaction: UPSERT (the cipherocto R15 fix path).
    // Must succeed and UPDATE the existing row.
    {
        let mut tx = db.begin().expect("begin tx2");
        tx.execute(
            "INSERT INTO app_state_mutation_macs
                (name, version, index_mac, value_mac, device_id)
             VALUES ($1, $2, $3, $4, $5)
             ON DUPLICATE KEY UPDATE
                 version = $2,
                 value_mac = $4",
            (
                "critical_block",
                2i64,
                stoolap::core::Value::blob(vec![0u8; 32]),
                stoolap::core::Value::blob(vec![0x22u8; 32]),
                42i64,
            ),
        )
        .expect("UPSERT in tx2 (R15 fix must apply ON DUPLICATE KEY UPDATE branch)");
        tx.commit().expect("commit tx2");
    }

    // Verify: exactly one row, value_mac updated to 0x22.
    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM app_state_mutation_macs", ())
        .expect("count");
    assert_eq!(count, 1, "UPSERT in tx must UPDATE, not INSERT a duplicate");

    let value_mac: Vec<u8> = db
        .query_one(
            "SELECT value_mac FROM app_state_mutation_macs
             WHERE name = $1 AND device_id = $2",
            ("critical_block", 42i64),
        )
        .expect("query");
    assert_eq!(
        value_mac,
        vec![0x22u8; 32],
        "value_mac must be the new one (0x22) after UPSERT in tx"
    );
}
