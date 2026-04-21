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

//! Aggregate Function Regression Tests Inside Transactions
//!
//! Tests that aggregate functions (SUM, COUNT, AVG, MIN, MAX) work correctly
//! when executed inside MVCC transactions.
//!
//! Related: RFC-0204 - Expression Compiler Aggregate Function Resolution
//!
//! # Bug Description
//!
//! When executing aggregate functions inside MVCC transactions (e.g.,
//! `SELECT SUM(amount) FROM accounts WHERE user_id = $1 FOR UPDATE`), stoolap
//! returns "Function not found: SUM" because the expression compiler only checks
//! the scalar function registry (`get_scalar()`), never checking the aggregate
//! registry (`get_aggregate()`).
//!
//! The same query outside transaction context succeeds because it routes through
//! the aggregation pushdown optimization (`try_aggregation_pushdown()`) which
//! bypasses expression compilation.

use stoolap::Database;

/// Setup a simple accounts table for testing
fn setup_accounts_table(db: &Database) {
    db.execute(
        "CREATE TABLE accounts (id INTEGER PRIMARY KEY, user_id INTEGER, amount FLOAT, status TEXT)",
        (),
    )
    .expect("Failed to create accounts table");

    // Insert test data
    let inserts = [
        "INSERT INTO accounts (id, user_id, amount, status) VALUES (1, 1, 100.0, 'active')",
        "INSERT INTO accounts (id, user_id, amount, status) VALUES (2, 1, 200.0, 'active')",
        "INSERT INTO accounts (id, user_id, amount, status) VALUES (3, 1, 150.0, 'inactive')",
        "INSERT INTO accounts (id, user_id, amount, status) VALUES (4, 2, 300.0, 'active')",
        "INSERT INTO accounts (id, user_id, amount, status) VALUES (5, 2, 450.0, 'active')",
    ];

    for insert in &inserts {
        db.execute(insert, ()).expect("Failed to insert test data");
    }
}

/// Test that SUM aggregate works inside a transaction
/// This is a regression test for the "Function not found: SUM" bug
/// inside MVCC transactions with FOR UPDATE.
#[test]
fn test_sum_inside_transaction() {
    let db = Database::open("memory://aggregate_sum_tx").expect("Failed to create database");
    setup_accounts_table(&db);

    // Start a transaction
    let mut tx = db.begin().expect("Failed to begin transaction");

    // This query fails with "Function not found: SUM" due to compiler bug
    // The FOR UPDATE clause disqualifies the query from aggregation pushdown,
    // forcing it through the expression compilation path where SUM is not found.
    // SUM on FLOAT column returns f64 in stoolap.
    let sum_result: Result<f64, _> = tx.query_one(
        "SELECT SUM(amount) FROM accounts WHERE user_id = 1 FOR UPDATE",
        (),
    );

    // After the fix, this should succeed. Currently it returns an error.
    match sum_result {
        Ok(sum) => {
            // user_id=1 has accounts with amounts: 100.0 + 200.0 + 150.0 = 450.0
            assert!((sum - 450.0).abs() < 0.001, "Expected sum of 450.0, got {}", sum);
        }
        Err(e) => {
            // Currently fails with: "Compile error: Function not found: SUM"
            // After RFC-0204 fix, this should not occur
            panic!("SUM inside transaction failed (BUG): {:?}", e);
        }
    }

    tx.commit().expect("Failed to commit transaction");
}

/// Test that COUNT aggregate works inside a transaction
#[test]
fn test_count_inside_transaction() {
    let db = Database::open("memory://aggregate_count_tx").expect("Failed to create database");
    setup_accounts_table(&db);

    let mut tx = db.begin().expect("Failed to begin transaction");

    let count_result: Result<i64, _> = tx.query_one(
        "SELECT COUNT(*) FROM accounts WHERE user_id = 1 FOR UPDATE",
        (),
    );

    match count_result {
        Ok(count) => {
            // user_id=1 has 3 accounts
            assert_eq!(count, 3, "Expected count of 3");
        }
        Err(e) => {
            panic!("COUNT inside transaction failed (BUG): {:?}", e);
        }
    }

    tx.commit().expect("Failed to commit transaction");
}

/// Test that AVG aggregate works inside a transaction
#[test]
fn test_avg_inside_transaction() {
    let db = Database::open("memory://aggregate_avg_tx").expect("Failed to create database");
    setup_accounts_table(&db);

    let mut tx = db.begin().expect("Failed to begin transaction");

    let avg_result: Result<f64, _> = tx.query_one(
        "SELECT AVG(amount) FROM accounts WHERE user_id = 1 FOR UPDATE",
        (),
    );

    match avg_result {
        Ok(avg) => {
            // (100.0 + 200.0 + 150.0) / 3 = 150.0
            assert!((avg - 150.0).abs() < 0.001, "Expected avg of 150.0, got {}", avg);
        }
        Err(e) => {
            panic!("AVG inside transaction failed (BUG): {:?}", e);
        }
    }

    tx.commit().expect("Failed to commit transaction");
}

/// Test that MIN/MAX aggregates work inside a transaction
#[test]
fn test_min_max_inside_transaction() {
    let db = Database::open("memory://aggregate_min_max_tx").expect("Failed to create database");
    setup_accounts_table(&db);

    let mut tx = db.begin().expect("Failed to begin transaction");

    // MIN and MAX return f64 in stoolap
    let min_result: Result<f64, _> = tx.query_one(
        "SELECT MIN(amount) FROM accounts WHERE user_id = 1 FOR UPDATE",
        (),
    );
    let max_result: Result<f64, _> = tx.query_one(
        "SELECT MAX(amount) FROM accounts WHERE user_id = 1 FOR UPDATE",
        (),
    );

    match (min_result, max_result) {
        (Ok(min), Ok(max)) => {
            assert!((min - 100.0).abs() < 0.001, "Expected min of 100.0, got {}", min);
            assert!((max - 200.0).abs() < 0.001, "Expected max of 200.0, got {}", max);
        }
        (Err(e), _) | (_, Err(e)) => {
            panic!("MIN/MAX inside transaction failed (BUG): {:?}", e);
        }
    }

    tx.commit().expect("Failed to commit transaction");
}

/// Test that aggregates work inside transaction WITHOUT FOR UPDATE
/// This should pass even before the fix (uses pushdown path)
#[test]
fn test_aggregates_no_for_update() {
    let db = Database::open("memory://aggregate_no_for_update").expect("Failed to create database");
    setup_accounts_table(&db);

    let mut tx = db.begin().expect("Failed to begin transaction");

    // Without FOR UPDATE, this should use aggregation pushdown and work
    // SUM on FLOAT returns f64
    let sum_result: Result<f64, _> = tx.query_one(
        "SELECT SUM(amount) FROM accounts WHERE user_id = 1",
        (),
    );
    assert!(sum_result.is_ok(), "SUM without FOR UPDATE should work (pushdown)");

    // COUNT(*) returns i64
    let count_result: Result<i64, _> = tx.query_one(
        "SELECT COUNT(*) FROM accounts WHERE user_id = 1",
        (),
    );
    assert!(count_result.is_ok(), "COUNT without FOR UPDATE should work (pushdown)");

    tx.commit().expect("Failed to commit transaction");
}

/// Test GROUP BY with aggregate inside transaction
#[test]
fn test_group_by_with_aggregate_in_transaction() {
    let db = Database::open("memory://aggregate_group_by_tx").expect("Failed to create database");
    setup_accounts_table(&db);

    let mut tx = db.begin().expect("Failed to begin transaction");

    let result = tx.query(
        "SELECT user_id, SUM(amount) FROM accounts GROUP BY user_id FOR UPDATE",
        (),
    );

    match result {
        Ok(rows) => {
            let mut results = Vec::new();
            for row in rows {
                let row = row.expect("Failed to get row");
                let user_id: i64 = row.get(0).unwrap();
                let sum: f64 = row.get(1).unwrap();
                results.push((user_id, sum));
            }
            // user_id=1: 100+200+150=450, user_id=2: 300+450=750
            assert!(results.contains(&(1, 450.0)), "Expected user_id=1 sum=450");
            assert!(results.contains(&(2, 750.0)), "Expected user_id=2 sum=750");
        }
        Err(e) => {
            panic!("GROUP BY with aggregate in transaction failed (BUG): {:?}", e);
        }
    }

    tx.commit().expect("Failed to commit transaction");
}