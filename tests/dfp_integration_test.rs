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

//! DFP (Deterministic Floating-Point) Integration Tests
//!
//! Tests DFP type with full SQL queries: CREATE → INSERT → SELECT → WHERE → UPDATE → DELETE

use stoolap::Database;

/// Test basic DFP column storage and retrieval
#[test]
fn test_dfp_basic_insert_select() {
    let db = Database::open("memory://dfp_basic").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_test (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    // Insert DFP values
    db.execute("INSERT INTO dfp_test VALUES (1, 3.14)", ())
        .expect("Failed to insert DFP value");
    db.execute("INSERT INTO dfp_test VALUES (2, 2.718)", ())
        .expect("Failed to insert DFP value");
    db.execute("INSERT INTO dfp_test VALUES (3, 1.414)", ())
        .expect("Failed to insert DFP value");

    // Verify we can read them back
    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM dfp_test", ())
        .expect("Failed to count");
    assert_eq!(count, 3, "Expected 3 rows");
}

/// Test DFP in WHERE clause comparison
#[test]
fn test_dfp_where_comparison() {
    let db = Database::open("memory://dfp_where").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_where (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_where VALUES (1, 1.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_where VALUES (2, 2.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_where VALUES (3, 3.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_where VALUES (4, 4.0)", ())
        .expect("Failed to insert");

    // WHERE value > 2.0 should return 2 rows (3.0 and 4.0)
    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM dfp_where WHERE value > 2.0", ())
        .expect("Failed to query");
    assert_eq!(count, 2, "Expected 2 rows with value > 2.0");
}

/// Test DFP arithmetic in SELECT expressions
#[test]
fn test_dfp_arithmetic_in_select() {
    let db = Database::open("memory://dfp_arith").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_arith (a DFP, b DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_arith VALUES (2.0, 3.0)", ())
        .expect("Failed to insert");

    // SELECT a * b should give 6.0
    let result: f64 = db
        .query_one("SELECT a * b FROM dfp_arith", ())
        .expect("Failed to query");
    assert!((result - 6.0).abs() < 0.001, "Expected 6.0, got {}", result);
}

/// Test DFP UPDATE
#[test]
fn test_dfp_update() {
    let db = Database::open("memory://dfp_update").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_update (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_update VALUES (1, 1.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_update VALUES (2, 2.0)", ())
        .expect("Failed to insert");

    // Update: set value = value * 2 where id = 1
    db.execute("UPDATE dfp_update SET value = value * 2.0 WHERE id = 1", ())
        .expect("Failed to update");

    let updated: f64 = db
        .query_one("SELECT value FROM dfp_update WHERE id = 1", ())
        .expect("Failed to query");
    assert!((updated - 2.0).abs() < 0.001, "Expected 2.0 after update, got {}", updated);
}

/// Test DFP DELETE
#[test]
fn test_dfp_delete() {
    let db = Database::open("memory://dfp_delete").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_delete (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_delete VALUES (1, 1.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_delete VALUES (2, 2.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_delete VALUES (3, 3.0)", ())
        .expect("Failed to insert");

    // Delete row where value < 2.0 (should delete only id=1)
    db.execute("DELETE FROM dfp_delete WHERE value < 2.0", ())
        .expect("Failed to delete");

    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM dfp_delete", ())
        .expect("Failed to count");
    assert_eq!(count, 2, "Expected 2 rows remaining after delete");
}

/// Test DFP with ORDER BY
#[test]
fn test_dfp_order_by() {
    let db = Database::open("memory://dfp_order").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_order (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_order VALUES (1, 3.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_order VALUES (2, 1.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_order VALUES (3, 2.0)", ())
        .expect("Failed to insert");

    // Query ordered by value ASC
    let result = db
        .query("SELECT id, value FROM dfp_order ORDER BY value ASC", ())
        .expect("Failed to query");

    let mut ids: Vec<i64> = Vec::new();
    for row in result {
        let row = row.expect("Failed to get row");
        ids.push(row.get(0).unwrap());
    }
    assert_eq!(ids, vec![2, 3, 1], "Expected ids in order 2, 3, 1");
}

/// Test DFP with aggregate functions
#[test]
fn test_dfp_aggregates() {
    eprintln!("TEST START: test_dfp_aggregates");
    let db = Database::open("memory://dfp_agg_new").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_agg (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_agg VALUES (1, 1.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_agg VALUES (2, 2.0)", ())
        .expect("Failed to insert");
    db.execute("INSERT INTO dfp_agg VALUES (3, 3.0)", ())
        .expect("Failed to insert");

    // SUM of values should be 6.0
    let sum: f64 = db
        .query_one("SELECT SUM(value) FROM dfp_agg", ())
        .expect("Failed to query");
    assert!((sum - 6.0).abs() < 0.001, "Expected SUM=6.0, got {}", sum);

    // AVG of values should be 2.0
    let avg: f64 = db
        .query_one("SELECT AVG(value) FROM dfp_agg", ())
        .expect("Failed to query");
    assert!((avg - 2.0).abs() < 0.001, "Expected AVG=2.0, got {}", avg);

    // COUNT should be 3
    let count: i64 = db
        .query_one("SELECT COUNT(value) FROM dfp_agg", ())
        .expect("Failed to query");
    assert_eq!(count, 3, "Expected COUNT=3");
}

/// Test DFP CAST from TEXT
#[test]
fn test_dfp_cast_from_text() {
    let db = Database::open("memory://dfp_cast").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_cast (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    db.execute("INSERT INTO dfp_cast VALUES (1, CAST('3.14159' AS DFP))", ())
        .expect("Failed to insert");

    let result: f64 = db
        .query_one("SELECT value FROM dfp_cast WHERE id = 1", ())
        .expect("Failed to query");
    assert!(
        (result - std::f64::consts::PI).abs() < 0.001,
        "Expected ~3.14159, got {}",
        result
    );
}

/// Test DFP round-trip (serialize → deserialize)
#[test]
fn test_dfp_roundtrip() {
    let db = Database::open("memory://dfp_rt").expect("Failed to create database");

    db.execute("CREATE TABLE dfp_rt (id INTEGER, value DFP)", ())
        .expect("Failed to create table");

    // Insert a value with many decimal places
    db.execute("INSERT INTO dfp_rt VALUES (1, 1.23456789012345)", ())
        .expect("Failed to insert");

    // Read it back and verify we get the same value
    let result: f64 = db
        .query_one("SELECT value FROM dfp_rt WHERE id = 1", ())
        .expect("Failed to query");
    assert!(
        (result - 1.23456789012345).abs() < 1e-10,
        "Expected ~1.23456789012345, got {}",
        result
    );
}
