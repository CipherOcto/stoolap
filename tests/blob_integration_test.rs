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

//! Integration tests for RFC-0201 Phase 2c: Blob in Projection/Selection
//!
//! Tests that BYTEA columns work correctly in SQL projection and selection contexts.

use stoolap::api::Database;

/// Test basic BYTEA insertion and retrieval via query
#[test]
fn test_blob_basic_insert_and_select() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE api_keys (id INTEGER PRIMARY KEY, key_hash BYTEA(32))",
        (),
    )
    .expect("Failed to create table");

    // SHA256 of "test_key_1" as raw bytes
    let key_hash = vec![
        0x9f, 0x86, 0xd0, 0x81, 0x88, 0x4c, 0x7d, 0x65, 0xa2, 0xfe, 0xaa, 0x0c, 0x55, 0xad,
        0x01, 0x5a, 0x3b, 0xf4, 0xf1, 0xb2, 0xb0, 0xb8, 0x22, 0xcd, 0x15, 0xd6, 0xc1, 0x5b,
        0x0f, 0x00, 0xa0, 0x08,
    ];
    assert_eq!(key_hash.len(), 32);

    // Insert using $1 parameter (Vec<u8> wrapped in tuple)
    db.execute("INSERT INTO api_keys (id, key_hash) VALUES (1, $1)", (key_hash.clone(),))
        .expect("Failed to insert blob");

    // Retrieve the blob back using query_one with Vec<u8>
    let result = db
        .query_one::<Vec<u8>, _>("SELECT key_hash FROM api_keys WHERE id = 1", ())
        .expect("Failed to query");

    assert_eq!(result, key_hash);
}

/// Test BYTEA in projection with multiple columns
#[test]
fn test_blob_projection_multiple_columns() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE usage_ledger (event_id INTEGER PRIMARY KEY, key_hash BYTEA(32), signature BYTEA(8))",
        (),
    )
    .expect("Failed to create table");

    let key_hash = vec![
        0x47, 0xea, 0x61, 0x59, 0xd8, 0xaf, 0x1a, 0x9e, 0x94, 0xf0, 0x61, 0x74, 0xbc, 0x6b,
        0x27, 0xc5, 0x3d, 0x2f, 0x0c, 0xf5, 0x8d, 0x2b, 0x0a, 0x1b, 0x6f, 0x8b, 0x0e, 0x0b,
        0x0a, 0x0b, 0x0b, 0x0b,
    ];
    let signature = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33];

    db.execute(
        "INSERT INTO usage_ledger (event_id, key_hash, signature) VALUES (1, $1, $2)",
        (key_hash.clone(), signature.clone()),
    )
    .expect("Failed to insert");

    // Query both BYTEA columns
    let result = db
        .query("SELECT key_hash, signature FROM usage_ledger WHERE event_id = 1", ())
        .expect("Failed to query");

    let mut count = 0;
    for row_result in result {
        let row = row_result.expect("Row error");
        let retrieved_hash: Vec<u8> = row.get(0).expect("Failed to get key_hash");
        let retrieved_sig: Vec<u8> = row.get(1).expect("Failed to get signature");
        assert_eq!(retrieved_hash, key_hash);
        assert_eq!(retrieved_sig, signature);
        count += 1;
    }
    assert_eq!(count, 1, "Expected exactly one row");
}

/// Test BYTEA equality in WHERE clause
#[test]
fn test_blob_equality_in_where() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE events (id INTEGER PRIMARY KEY, event_id BYTEA(32))",
        (),
    )
    .expect("Failed to create table");

    let event1 = vec![
        0x47, 0xea, 0x61, 0x59, 0xd8, 0xaf, 0x1a, 0x9e, 0x94, 0xf0, 0x61, 0x74, 0xbc, 0x6b,
        0x27, 0xc5, 0x3d, 0x2f, 0x0c, 0xf5, 0x8d, 0x2b, 0x0a, 0x1b, 0x6f, 0x8b, 0x0e, 0x0b,
        0x0a, 0x0b, 0x0b, 0x0b,
    ];
    let event2 = vec![
        0x9f, 0x86, 0xd0, 0x81, 0x88, 0x4c, 0x7d, 0x65, 0xa2, 0xfe, 0xaa, 0x0c, 0x55, 0xad,
        0x01, 0x5a, 0x3b, 0xf4, 0xf1, 0xb2, 0xb0, 0xb8, 0x22, 0xcd, 0x15, 0xd6, 0xc1, 0x5b,
        0x0f, 0x00, 0xa0, 0x08,
    ];

    db.execute("INSERT INTO events VALUES (1, $1)", (event1.clone(),))
        .expect("Failed to insert event1");
    db.execute("INSERT INTO events VALUES (2, $1)", (event2.clone(),))
        .expect("Failed to insert event2");

    // Query by exact event_id match
    let result = db
        .query_one::<Vec<u8>, _>("SELECT event_id FROM events WHERE id = 1", ())
        .expect("Failed to query");

    assert_eq!(result, event1);
}

/// Test BYTEA with fixed array syntax
#[test]
fn test_blob_fixed_length_syntax() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE fixed_blobs (id INTEGER PRIMARY KEY, data BYTEA(16))",
        (),
    )
    .expect("Failed to create table");

    let data = vec![0xDEu8; 16];

    db.execute("INSERT INTO fixed_blobs (id, data) VALUES (1, $1)", (data.clone(),))
        .expect("Failed to insert");

    let result = db
        .query_one::<Vec<u8>, _>("SELECT data FROM fixed_blobs WHERE id = 1", ())
        .expect("Failed to query");

    assert_eq!(result, data);
}

/// Test BYTEA as part of ORDER BY
#[test]
fn test_blob_order_by() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE blob_order (id INTEGER PRIMARY KEY, blob_key BYTEA(32))",
        (),
    )
    .expect("Failed to create table");

    // Insert in non-sorted order
    let key1 = vec![1u8; 32];
    let key2 = vec![2u8; 32];
    let key3 = vec![3u8; 32];

    db.execute("INSERT INTO blob_order VALUES (1, $1)", (key1.clone(),))
        .expect("Failed");
    db.execute("INSERT INTO blob_order VALUES (2, $1)", (key2.clone(),))
        .expect("Failed");
    db.execute("INSERT INTO blob_order VALUES (3, $1)", (key3.clone(),))
        .expect("Failed");

    // Query ordered by blob (byte comparison)
    let result = db
        .query("SELECT id FROM blob_order ORDER BY blob_key", ())
        .expect("Failed to query");

    let mut ids: Vec<i64> = Vec::new();
    for row_result in result {
        let row = row_result.expect("Row error");
        let id: i64 = row.get(0).expect("Failed to get id");
        ids.push(id);
    }

    assert_eq!(ids, vec![1, 2, 3]);
}

/// Test non-UTF8 blob data (binary data that would fail UTF-8 validation)
#[test]
fn test_blob_binary_data() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE binary_data (id INTEGER PRIMARY KEY, raw BYTEA)",
        (),
    )
    .expect("Failed to create table");

    // Binary data with null bytes and high values - NOT valid UTF-8
    let binary = vec![0x00, 0xFF, 0xFE, 0x00, 0x42, b'\0', 0x80, 0xFF];

    db.execute("INSERT INTO binary_data VALUES (1, $1)", (binary.clone(),))
        .expect("Failed to insert binary data");

    let result = db
        .query_one::<Vec<u8>, _>("SELECT raw FROM binary_data WHERE id = 1", ())
        .expect("Failed to query");

    assert_eq!(result, binary);
}

/// Test multiple BYTEA columns in same table with TEXT
#[test]
fn test_blob_mixed_with_text() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE mixed_cols (id INTEGER PRIMARY KEY, name TEXT, key_hash BYTEA(32))",
        (),
    )
    .expect("Failed to create table");

    let key_hash = vec![
        0x9f, 0x86, 0xd0, 0x81, 0x88, 0x4c, 0x7d, 0x65, 0xa2, 0xfe, 0xaa, 0x0c, 0x55, 0xad,
        0x01, 0x5a, 0x3b, 0xf4, 0xf1, 0xb2, 0xb0, 0xb8, 0x22, 0xcd, 0x15, 0xd6, 0xc1, 0x5b,
        0x0f, 0x00, 0xa0, 0x08,
    ];

    db.execute(
        "INSERT INTO mixed_cols (id, name, key_hash) VALUES (1, 'test_key', $1)",
        (key_hash.clone(),),
    )
    .expect("Failed to insert");

    // Query both TEXT and BYTEA
    let result = db
        .query("SELECT name, key_hash FROM mixed_cols WHERE id = 1", ())
        .expect("Failed to query");

    let mut count = 0;
    for row_result in result {
        let row = row_result.expect("Row error");
        let retrieved_name: String = row.get(0).expect("Failed to get name");
        let retrieved_hash: Vec<u8> = row.get(1).expect("Failed to get key_hash");
        assert_eq!(retrieved_name, "test_key");
        assert_eq!(retrieved_hash, key_hash);
        count += 1;
    }
    assert_eq!(count, 1, "Expected exactly one row");
}
