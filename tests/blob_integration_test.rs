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

/// Test direct Value::Blob comparison
#[test]
fn test_blob_direct_comparison() {
    use stoolap::core::Value;

    let blob1 = Value::blob(vec![0x01u8; 32]);
    let blob2 = Value::blob(vec![0x01u8; 32]);
    let blob3 = Value::blob(vec![0x02u8; 32]);

    // Same blobs should be equal
    assert_eq!(blob1, blob2, "Same blob values should be equal");

    // Different blobs should not be equal
    assert_ne!(blob1, blob3, "Different blob values should not be equal");

    // Verify PartialEq gives true for equal blobs
    assert!(blob1 == blob2);
    assert!(blob1 != blob3);
}

/// Test that blob round-trip (insert -> retrieve -> compare) works
#[test]
fn test_blob_round_trip() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v BYTEA)", ())
        .expect("Failed to create table");

    let original = vec![0x01u8; 32];
    db.execute("INSERT INTO t VALUES (1, $1)", (original.clone(),))
        .expect("Failed to insert");

    // Retrieve the blob
    let retrieved: Vec<u8> = db
        .query_one("SELECT v FROM t WHERE id = 1", ())
        .expect("Failed to retrieve");

    assert_eq!(retrieved, original, "Retrieved blob should match original");

    // Now test if we can find it by comparing with original
    let ids: Vec<i64> = db
        .query("SELECT id FROM t WHERE v = $1", (original.clone(),))
        .unwrap()
        .map(|r| r.unwrap().get::<i64>(0).unwrap())
        .collect();

    assert_eq!(ids, vec![1], "Should find blob by value");
}

/// Test blob comparison with blob retrieved from a Row
#[test]
fn test_blob_row_comparison() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v BYTEA)", ())
        .expect("Failed to create table");

    let original = vec![0x01u8; 32];
    db.execute("INSERT INTO t VALUES (1, $1)", (original.clone(),))
        .expect("Failed to insert");

    // Retrieve the blob value directly
    let retrieved_blob: Vec<u8> = db
        .query_one("SELECT v FROM t WHERE id = 1", ())
        .expect("Failed to query");

    assert_eq!(retrieved_blob, original);

    // Now try to use that blob value in a comparison
    let ids: Vec<i64> = db
        .query("SELECT id FROM t WHERE v = $1", (retrieved_blob,))
        .unwrap()
        .map(|r| r.unwrap().get::<i64>(0).unwrap())
        .collect();

    assert_eq!(ids, vec![1], "Should find blob by value from row");
}

/// Debug test to see what's happening with blob comparison
#[test]
fn test_blob_debug_comparison() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute("CREATE TABLE t (id INTEGER, v BYTEA)", ())
        .expect("Failed to create table");

    let original = vec![0x01u8; 32];

    // Insert
    db.execute("INSERT INTO t VALUES (1, $1)", (original.clone(),))
        .expect("Failed to insert");

    // Check EXPLAIN for the blob query
    let explain_count = db.query("EXPLAIN QUERY PLAN SELECT id FROM t WHERE v = $1", (original.clone(),))
        .map(|r| r.count()).unwrap_or(0);
    println!("EXPLAIN blob query count: {}", explain_count);

    // Check EXPLAIN for integer query
    let explain_int_count = db.query("EXPLAIN QUERY PLAN SELECT id FROM t WHERE id = $1", (1,))
        .map(|r| r.count()).unwrap_or(0);
    println!("EXPLAIN int query count: {}", explain_int_count);

    // Select all without WHERE
    let all_rows = db.query("SELECT id, v FROM t", ()).unwrap();
    for row in all_rows {
        let row = row.unwrap();
        let id: i64 = row.get(0).unwrap();
        let v: Vec<u8> = row.get(1).unwrap();
        println!("All rows - id={}, v len={}", id, v.len());
    }

    // Test with a filter that's always true
    let always_true = db.query("SELECT id FROM t WHERE 1=1", ());
    println!("Always true filter: {:?}", always_true.map(|r| r.count()));

    // Test with v IS NOT NULL
    let not_null = db.query("SELECT id FROM t WHERE v IS NOT NULL", ());
    println!("v IS NOT NULL: {:?}", not_null.map(|r| r.count()));

    // Test with id = 1 (integer comparison should work)
    let by_int = db.query("SELECT id FROM t WHERE id = 1", ());
    println!("id = 1: {:?}", by_int.map(|r| r.count()));

    // Test with v = $1 (blob param)
    let by_blob = db.query("SELECT id FROM t WHERE v = $1", (original.clone(),));
    println!("v = $1 (blob): {:?}", by_blob.map(|r| r.count()));

    // Test by first selecting the blob value and then using it
    let retrieved: Vec<u8> = db.query_one("SELECT v FROM t WHERE id = 1", ()).unwrap();
    println!("Retrieved blob len={}", retrieved.len());

    // Use the retrieved blob in a query
    let by_retrieved = db.query("SELECT id FROM t WHERE v = $1", (retrieved,));
    println!("v = $1 (retrieved): {:?}", by_retrieved.map(|r| r.count()));
}

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

    // First verify data was inserted - get all rows
    let result = db
        .query("SELECT id FROM events", ())
        .expect("Failed to query");

    let mut count = 0;
    for row_result in result {
        let _row = row_result.expect("Row error");
        count += 1;
    }
    assert_eq!(count, 2, "Should have 2 rows");

    // Query by integer id (works)
    let result: Vec<i64> = db
        .query("SELECT id FROM events WHERE id = 1", ())
        .expect("Failed to query")
        .map(|r| r.unwrap().get::<i64>(0).unwrap())
        .collect();
    assert_eq!(result, vec![1]);
}

/// Test BYTEA parameter comparison in WHERE clause (separate test to isolate issue)
#[test]
fn test_blob_param_comparison() {
    let db = Database::open_in_memory().expect("Failed to create database");

    // First test with TEXT to verify TEXT parameter comparison works
    db.execute("CREATE TABLE text_test (id INTEGER PRIMARY KEY, val TEXT)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO text_test VALUES (1, 'hello')", ())
        .expect("Failed to insert text");

    let text_result: Vec<i64> = db
        .query("SELECT id FROM text_test WHERE val = $1", ("hello",))
        .unwrap()
        .map(|r| r.unwrap().get::<i64>(0).unwrap())
        .collect();
    assert_eq!(text_result, vec![1], "TEXT param comparison should work");

    // Now test with BYTEA
    db.execute("CREATE TABLE events (id INTEGER PRIMARY KEY, event_id BYTEA)", ())
        .expect("Failed to create table");

    let event1 = vec![0x01u8; 32];

    db.execute("INSERT INTO events VALUES (1, $1)", (event1.clone(),))
        .expect("Failed to insert");

    // Step 1: Verify data is there by selecting all
    let all_result = db
        .query("SELECT id, event_id FROM events", ())
        .expect("Failed to query all");
    let mut all_count = 0;
    for r in all_result {
        let _row = r.expect("Row error");
        all_count += 1;
    }
    assert_eq!(all_count, 1, "Step 1: Should have 1 row total");

    // Step 2: Try WHERE id = 1 (integer comparison with literal)
    let result = db
        .query("SELECT id FROM events WHERE id = 1", ())
        .expect("Step 2a: Failed to query");
    let mut count = 0;
    for _row_result in result {
        count += 1;
    }
    assert_eq!(count, 1, "Step 2a: Should find row by id=1");

    // Step 3: Try WHERE id = $1 with integer param
    let result = db
        .query("SELECT id FROM events WHERE id = $1", (1,))
        .expect("Step 3: Failed to query");
    let mut count = 0;
    for _row_result in result {
        count += 1;
    }
    assert_eq!(count, 1, "Step 3: Should find row by id=$1 (integer param)");

    // Step 4: Try WHERE event_id = $1 with Vec<u8> param
    let result = db
        .query("SELECT id FROM events WHERE event_id = $1", (event1.clone(),))
        .expect("Step 4: Failed to query");
    let mut ids: Vec<i64> = Vec::new();
    for row_result in result {
        let row = row_result.expect("Step 4: Row error");
        let id: i64 = row.get(0).expect("Step 4: Failed to get id");
        ids.push(id);
    }
    assert_eq!(ids, vec![1], "Step 4: Should find row by blob equality, got: {:?}", ids);
}

/// Test BYTEA inequality in WHERE clause
#[test]
fn test_blob_inequality_in_where() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute("CREATE TABLE events (id INTEGER PRIMARY KEY, event_id BYTEA)", ())
        .expect("Failed to create table");

    let event1 = vec![0x01u8; 32];
    let event2 = vec![0x02u8; 32];
    let event3 = vec![0x03u8; 32];

    db.execute("INSERT INTO events VALUES (1, $1)", (event1.clone(),))
        .expect("Failed to insert");
    db.execute("INSERT INTO events VALUES (2, $1)", (event2.clone(),))
        .expect("Failed to insert");
    db.execute("INSERT INTO events VALUES (3, $1)", (event3.clone(),))
        .expect("Failed to insert");

    // Query for event_id NOT equal to event1
    let result = db
        .query("SELECT id FROM events WHERE event_id <> $1", (event1.clone(),))
        .expect("Failed to query");

    let mut ids: Vec<i64> = Vec::new();
    for row_result in result {
        let row = row_result.expect("Row error");
        let id: i64 = row.get(0).expect("Failed to get id");
        ids.push(id);
    }
    ids.sort();
    assert_eq!(ids, vec![2, 3]);

    // Query for event_id greater than event1
    let result = db
        .query("SELECT id FROM events WHERE event_id > $1", (event1.clone(),))
        .expect("Failed to query");

    let mut ids: Vec<i64> = Vec::new();
    for row_result in result {
        let row = row_result.expect("Row error");
        let id: i64 = row.get(0).expect("Failed to get id");
        ids.push(id);
    }
    ids.sort();
    assert_eq!(ids, vec![2, 3]);
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

/// Test hash index on BYTEA column
#[test]
fn test_hash_index_on_blob_column() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE api_keys (id INTEGER PRIMARY KEY, key_hash BYTEA(32))",
        (),
    )
    .expect("Failed to create table");

    // Create hash index on BYTEA column
    db.execute("CREATE INDEX idx_hash ON api_keys(key_hash) USING HASH", ())
        .expect("Failed to create hash index");

    let key1 = vec![0x01u8; 32];
    let key2 = vec![0x02u8; 32];

    db.execute("INSERT INTO api_keys VALUES (1, $1)", (key1.clone(),))
        .unwrap();
    db.execute("INSERT INTO api_keys VALUES (2, $1)", (key2.clone(),))
        .unwrap();

    // Lookup by blob value - should use hash index
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM api_keys WHERE key_hash = $1",
            (key1.clone(),),
        )
        .expect("Failed to query");

    assert_eq!(count, 1, "Should find exactly 1 row by blob hash");
    
    // Lookup the second key
    let count2: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM api_keys WHERE key_hash = $1",
            (key2.clone(),),
        )
        .expect("Failed to query");
        
    assert_eq!(count2, 1, "Should find exactly 1 row for key2");
    
    // Non-existent key
    let key3 = vec![0x03u8; 32];
    let count3: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM api_keys WHERE key_hash = $1",
            (key3,),
        )
        .expect("Failed to query");
        
    assert_eq!(count3, 0, "Should not find non-existent key");
}
