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

//! Integration tests for BIGINT and DECIMAL types per RFC-0202
//!
//! Tests the full SQL parsing path: typed literals (BIGINT '123', DECIMAL '1.5'),
//! CAST expressions, and basic operations.

use stoolap::Database;

#[test]
fn test_bigint_typed_literal() {
    let db = Database::open("memory://test_bigint_typed_literal").unwrap();

    // BIGINT typed literal in SELECT
    let result: i64 = db.query_one("SELECT BIGINT '12345'", ()).unwrap();
    assert_eq!(result, 12345);
}

#[test]
fn test_bigint_typed_literal_negative() {
    let db = Database::open("memory://test_bigint_typed_literal_neg").unwrap();

    // Negative BIGINT typed literal
    let result: i64 = db.query_one("SELECT BIGINT '-98765'", ()).unwrap();
    assert_eq!(result, -98765);
}

#[test]
fn test_decimal_typed_literal() {
    let db = Database::open("memory://test_decimal_typed_literal").unwrap();

    // DECIMAL typed literal
    let result: f64 = db.query_one("SELECT DECIMAL '123.45'", ()).unwrap();
    assert!((result - 123.45).abs() < 0.001);
}

#[test]
fn test_decimal_typed_literal_scale() {
    let db = Database::open("memory://test_decimal_typed_literal_scale").unwrap();

    // DECIMAL with different scales
    let result: f64 = db.query_one("SELECT DECIMAL '10.5'", ()).unwrap();
    assert!((result - 10.5).abs() < 0.001);
}

#[test]
fn test_cast_to_bigint() {
    let db = Database::open("memory://test_cast_to_bigint").unwrap();

    db.execute("CREATE TABLE nums (value TEXT)", ()).unwrap();
    db.execute("INSERT INTO nums VALUES ('12345')", ()).unwrap();

    // CAST TEXT to BIGINT
    let result: i64 = db
        .query_one("SELECT CAST(value AS BIGINT) FROM nums", ())
        .unwrap();
    assert_eq!(result, 12345);
}

#[test]
fn test_cast_to_decimal() {
    let db = Database::open("memory://test_cast_to_decimal").unwrap();

    db.execute("CREATE TABLE nums (value TEXT)", ()).unwrap();
    db.execute("INSERT INTO nums VALUES ('123.45')", ())
        .unwrap();

    // CAST TEXT to DECIMAL
    let result: f64 = db
        .query_one("SELECT CAST(value AS DECIMAL) FROM nums", ())
        .unwrap();
    assert!((result - 123.45).abs() < 0.001);
}

#[test]
fn test_bigint_column_creation() {
    let db = Database::open("memory://test_bigint_column").unwrap();

    // Create table with BIGINT column
    db.execute(
        "CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance BIGINT)",
        (),
    )
    .unwrap();

    db.execute("INSERT INTO accounts VALUES (1, 1000000000000)", ())
        .unwrap();
    db.execute("INSERT INTO accounts VALUES (2, 2000000000000)", ())
        .unwrap();

    let result = db
        .query("SELECT balance FROM accounts ORDER BY id", ())
        .unwrap();
    let rows: Vec<i64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert_eq!(rows, vec![1000000000000i64, 2000000000000i64]);
}

#[test]
fn test_decimal_column_creation() {
    let db = Database::open("memory://test_decimal_column").unwrap();

    // Create table with DECIMAL column
    db.execute(
        "CREATE TABLE products (id INTEGER PRIMARY KEY, price DECIMAL)",
        (),
    )
    .unwrap();

    db.execute("INSERT INTO products VALUES (1, 19.99)", ())
        .unwrap();
    db.execute("INSERT INTO products VALUES (2, 99.99)", ())
        .unwrap();

    let result = db
        .query("SELECT price FROM products ORDER BY id", ())
        .unwrap();
    let rows: Vec<f64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert!((rows[0] - 19.99).abs() < 0.001);
    assert!((rows[1] - 99.99).abs() < 0.001);
}

#[test]
fn test_bigint_arithmetic() {
    let db = Database::open("memory://test_bigint_arithmetic").unwrap();

    // BIGINT arithmetic
    let result: i64 = db
        .query_one("SELECT BIGINT '1000000000000' + BIGINT '2000000000000'", ())
        .unwrap();
    assert_eq!(result, 3000000000000i64);
}

#[test]
fn test_decimal_arithmetic() {
    let db = Database::open("memory://test_decimal_arithmetic").unwrap();

    // DECIMAL arithmetic
    let result: f64 = db
        .query_one("SELECT DECIMAL '10.5' + DECIMAL '20.3'", ())
        .unwrap();
    assert!((result - 30.8).abs() < 0.001);
}

#[test]
fn test_bigint_comparison() {
    let db = Database::open("memory://test_bigint_comparison").unwrap();

    // BIGINT comparison
    let result: bool = db
        .query_one("SELECT BIGINT '100' > BIGINT '50'", ())
        .unwrap();
    assert!(result);
}

#[test]
fn test_cross_type_comparison_integer_bigint() {
    let db = Database::open("memory://test_cross_type_cmp_ib").unwrap();

    // INTEGER vs BIGINT comparison
    let result: bool = db.query_one("SELECT 10 > BIGINT '5'", ()).unwrap();
    assert!(result);
}

#[test]
fn test_cross_type_comparison_integer_decimal() {
    let db = Database::open("memory://test_cross_type_cmp_id").unwrap();

    // INTEGER vs DECIMAL comparison
    let result: bool = db.query_one("SELECT 10 > DECIMAL '5.5'", ()).unwrap();
    assert!(result);
}

#[test]
fn test_cross_type_comparison_bigint_decimal() {
    let db = Database::open("memory://test_cross_type_cmp_bd").unwrap();

    // BIGINT vs DECIMAL comparison
    let result: bool = db
        .query_one("SELECT BIGINT '100' > DECIMAL '50.5'", ())
        .unwrap();
    assert!(result);
}

#[test]
fn test_bigint_sum_aggregate() {
    let db = Database::open("memory://test_bigint_sum_agg").unwrap();

    db.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, amount BIGINT)",
        (),
    )
    .unwrap();
    db.execute("INSERT INTO orders VALUES (1, 1000)", ())
        .unwrap();
    db.execute("INSERT INTO orders VALUES (2, 2000)", ())
        .unwrap();
    db.execute("INSERT INTO orders VALUES (3, 3000)", ())
        .unwrap();

    let result: i64 = db.query_one("SELECT SUM(amount) FROM orders", ()).unwrap();
    assert_eq!(result, 6000);
}

#[test]
fn test_decimal_avg_aggregate() {
    let db = Database::open("memory://test_decimal_avg_agg").unwrap();

    db.execute(
        "CREATE TABLE products (id INTEGER PRIMARY KEY, price DECIMAL)",
        (),
    )
    .unwrap();
    db.execute("INSERT INTO products VALUES (1, 10.0)", ())
        .unwrap();
    db.execute("INSERT INTO products VALUES (2, 20.0)", ())
        .unwrap();
    db.execute("INSERT INTO products VALUES (3, 30.0)", ())
        .unwrap();

    let result: f64 = db.query_one("SELECT AVG(price) FROM products", ()).unwrap();
    assert!((result - 20.0).abs() < 0.001);
}

#[test]
fn test_bigint_count_aggregate() {
    let db = Database::open("memory://test_bigint_count_agg").unwrap();

    db.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, amount BIGINT)",
        (),
    )
    .unwrap();
    db.execute("INSERT INTO orders VALUES (1, 1000)", ())
        .unwrap();
    db.execute("INSERT INTO orders VALUES (2, 2000)", ())
        .unwrap();
    db.execute("INSERT INTO orders VALUES (3, 3000)", ())
        .unwrap();

    let result: i64 = db.query_one("SELECT COUNT(*) FROM orders", ()).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_bigint_division_by_zero() {
    let db = Database::open("memory://test_bigint_div_zero").unwrap();

    // BIGINT division by zero should return error
    let result = db.query("SELECT BIGINT '10' / BIGINT '0'", ());
    assert!(result.is_err());
}

#[test]
fn test_decimal_division_by_zero() {
    let db = Database::open("memory://test_decimal_div_zero").unwrap();

    // DECIMAL division by zero should return error
    let result = db.query("SELECT DECIMAL '10.5' / DECIMAL '0'", ());
    assert!(result.is_err());
}

#[test]
fn test_bigint_null_handling() {
    let db = Database::open("memory://test_bigint_null").unwrap();

    db.execute("CREATE TABLE t (a BIGINT)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (10)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (NULL)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (20)", ()).unwrap();

    // NULL values should be ignored in aggregates
    let result: i64 = db.query_one("SELECT SUM(a) FROM t", ()).unwrap();
    assert_eq!(result, 30);
}

#[test]
fn test_decimal_null_handling() {
    let db = Database::open("memory://test_decimal_null").unwrap();

    db.execute("CREATE TABLE t (a DECIMAL)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (10.5)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (NULL)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (20.5)", ()).unwrap();

    // NULL values should be ignored in aggregates
    let result: f64 = db.query_one("SELECT SUM(a) FROM t", ()).unwrap();
    assert!((result - 31.0).abs() < 0.001);
}

#[test]
fn test_bigint_btree_index_ordering() {
    let db = Database::open("memory://test_bigint_btree_order").unwrap();

    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val BIGINT)", ())
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, 100)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (2, -50)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (3, 0)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (4, -100)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (5, 50)", ()).unwrap();

    // Create B-tree index on BIGINT column
    db.execute("CREATE INDEX idx_val ON t (val)", ()).unwrap();

    // Test ORDER BY ASC
    let result = db.query("SELECT id FROM t ORDER BY val ASC", ()).unwrap();
    let ids: Vec<i64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert_eq!(ids, vec![4, 2, 3, 5, 1]); // -100, -50, 0, 50, 100

    // Test ORDER BY DESC
    let result = db.query("SELECT id FROM t ORDER BY val DESC", ()).unwrap();
    let ids: Vec<i64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert_eq!(ids, vec![1, 5, 3, 2, 4]); // 100, 50, 0, -50, -100

    // Test range query using index
    let count: i64 = db
        .query_one("SELECT COUNT(*) FROM t WHERE val >= 0 AND val <= 100", ())
        .unwrap();
    assert_eq!(count, 3); // 0, 50, 100
}

#[test]
fn test_decimal_btree_index_ordering() {
    let db = Database::open("memory://test_decimal_btree_order").unwrap();

    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val DECIMAL)", ())
        .unwrap();
    db.execute("INSERT INTO t VALUES (1, 100.5)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (2, -50.5)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (3, 0.0)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (4, -100.5)", ()).unwrap();
    db.execute("INSERT INTO t VALUES (5, 50.5)", ()).unwrap();

    // Create B-tree index on DECIMAL column
    db.execute("CREATE INDEX idx_val ON t (val)", ()).unwrap();

    // Test ORDER BY ASC
    let result = db.query("SELECT id FROM t ORDER BY val ASC", ()).unwrap();
    let ids: Vec<i64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert_eq!(ids, vec![4, 2, 3, 5, 1]); // -100.5, -50.5, 0.0, 50.5, 100.5

    // Test ORDER BY DESC
    let result = db.query("SELECT id FROM t ORDER BY val DESC", ()).unwrap();
    let ids: Vec<i64> = result.map(|r| r.unwrap().get(0).unwrap()).collect();
    assert_eq!(ids, vec![1, 5, 3, 2, 4]); // 100.5, 50.5, 0.0, -50.5, -100.5
}
