// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.

//! Round 7 cipherocto review R7-F2 + stoolap composite-PK UPDATE
//! scope guard test.
//!
//! Original bug: UPDATE WHERE event_id = X matched all rows sharing
//! event_id=X across distinct recorder_did values, instead of only
//! the (recorder_did, event_id) composite-key row. This caused a
//! cross-recorder side effect where anchoring event 1 for Recorder A
//! also anchored it for Recorder B.
//!
//! Fix: stoolap now rejects UPDATE WHERE on a composite-PK or
//! non-INTEGER single-column-PK table unless the WHERE clause
//! references every PK column with equality. The composite-PK is
//! detected via the per-column `primary_key = true` flag set by the
//! DDL parser when processing `PRIMARY KEY (a, b)` table constraints.
//!
//! This file documents the reproduction and the post-fix contract:
//!   * `update_with_partial_pk_where_is_rejected` — UPDATE on the
//!     partial PK now returns Error::InvalidArgument.
//!   * `update_with_full_pk_where_succeeds_and_anchors_only_one` —
//!     UPDATE with full PK match anchors exactly one row.
//!   * `update_on_single_integer_pk_unaffected` — single INTEGER PK
//!     tables keep their old fast-path behavior (no WHERE constraint).

use stoolap::Database;

fn setup_composite_pk_table(tag: &str) -> Database {
    let db = Database::open(&format!("memory://anchor_update_repro_{tag}"))
        .expect("Failed to create database");
    db.execute(
        "CREATE TABLE reputation_events (
            recorder_did BLOB(52),
            event_id BLOB(8),
            controller_id BLOB(32),
            anchor_tx_hash BLOB,
            PRIMARY KEY (recorder_did, event_id)
        )",
        (),
    )
    .expect("create table");
    db
}

fn seed_two_rows(db: &Database) {
    let rec_a: Vec<u8> = vec![0xAA; 52];
    let rec_b: Vec<u8> = vec![0xBB; 52];
    let eid: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1];
    let cid: Vec<u8> = vec![0u8; 32];
    db.execute(
        "INSERT INTO reputation_events
         (recorder_did, event_id, controller_id, anchor_tx_hash)
         VALUES ($1, $2, $3, NULL)",
        stoolap::params![rec_a, eid.clone(), cid.clone()],
    )
    .expect("insert A");
    db.execute(
        "INSERT INTO reputation_events
         (recorder_did, event_id, controller_id, anchor_tx_hash)
         VALUES ($1, $2, $3, NULL)",
        stoolap::params![rec_b, eid, cid],
    )
    .expect("insert B");
}

#[test]
fn update_with_partial_pk_where_is_rejected() {
    let db = setup_composite_pk_table("rejected");
    seed_two_rows(&db);
    let anchor: Vec<u8> = vec![0xAA; 32];
    let eid: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1];
    let result = db.execute(
        "UPDATE reputation_events
         SET anchor_tx_hash = $1
         WHERE event_id = $2
           AND (anchor_tx_hash IS NULL OR anchor_tx_hash = $1)",
        stoolap::params![anchor, eid],
    );
    let err = result.expect_err("partial-PK WHERE must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("primary key") && msg.contains("recorder_did") && msg.contains("event_id"),
        "error must name both PK columns: {msg}"
    );
}

#[test]
fn update_with_full_pk_where_succeeds_and_anchors_only_one() {
    let db = setup_composite_pk_table("full");
    seed_two_rows(&db);
    let rec_a: Vec<u8> = vec![0xAA; 52];
    let eid: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1];
    let anchor: Vec<u8> = vec![0xAA; 32];
    let updated = db
        .execute(
            "UPDATE reputation_events
             SET anchor_tx_hash = $1
             WHERE recorder_did = $2 AND event_id = $3
             AND (anchor_tx_hash IS NULL OR anchor_tx_hash = $1)",
            stoolap::params![anchor.clone(), rec_a, eid],
        )
        .expect("full-PK UPDATE must succeed");
    assert_eq!(updated, 1, "exactly one row must be updated");
}

#[test]
fn update_on_single_integer_pk_unaffected() {
    // Single INTEGER PK keeps the rowid fast-path; any WHERE is OK.
    let db =
        Database::open("memory://single_int_pk_unaffected_v2").expect("Failed to create database");
    db.execute(
        "CREATE TABLE t (
            id INTEGER PRIMARY KEY,
            val TEXT,
            UNIQUE(id)
        )",
        (),
    )
    .expect("create");
    db.execute("INSERT INTO t (id, val) VALUES (1, 'a'), (2, 'b')", ())
        .expect("insert");
    let updated = db
        .execute("UPDATE t SET val = 'A' WHERE id = 1", ())
        .expect("single INTEGER PK must allow any WHERE");
    assert_eq!(updated, 1);
}
