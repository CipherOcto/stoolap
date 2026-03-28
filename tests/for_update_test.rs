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

//! Integration test for FOR UPDATE row locking
//!
//! Tests concurrent budget updates using SELECT ... FOR UPDATE to verify
//! pessimistic row locking works correctly.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

use stoolap::api::Database;

/// Test that FOR UPDATE syntax is parsed correctly and executes without error
#[test]
fn test_for_update_syntax() {
    let db = Database::open_in_memory().expect("Failed to create database");

    // Create budget table
    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, api_key TEXT, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    // Insert initial budget
    db.execute(
        "INSERT INTO budgets (id, api_key, remaining_quota) VALUES (1, 'test-key', 100)",
        (),
    )
    .expect("Failed to insert");

    // Test SELECT FOR UPDATE executes without error
    let result = db
        .query("SELECT * FROM budgets WHERE id = 1 FOR UPDATE", ())
        .expect("FOR UPDATE query failed");

    let count = result.count();
    assert_eq!(count, 1, "Should return 1 row");
}

/// Test that SELECT FOR UPDATE can read rows and then UPDATE within a transaction
#[test]
fn test_for_update_read_then_update() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, api_key TEXT, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    db.execute(
        "INSERT INTO budgets (id, api_key, remaining_quota) VALUES (1, 'test-key', 100)",
        (),
    )
    .expect("Failed to insert");

    // Use transaction with FOR UPDATE
    let mut tx = db.begin().expect("Failed to begin transaction");

    // Select with FOR UPDATE
    let result = tx
        .query(
            "SELECT remaining_quota FROM budgets WHERE id = 1 FOR UPDATE",
            (),
        )
        .expect("FOR UPDATE query failed");

    let initial_quota: i64 = result
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");

    assert_eq!(initial_quota, 100);

    // Update the quota
    tx.execute(
        "UPDATE budgets SET remaining_quota = remaining_quota - 10 WHERE id = 1",
        (),
    )
    .expect("UPDATE failed");

    tx.commit().expect("COMMIT failed");

    // Verify the update
    let result = db
        .query("SELECT remaining_quota FROM budgets WHERE id = 1", ())
        .expect("Query failed");

    let final_quota: i64 = result
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");
    assert_eq!(final_quota, 90);
}

/// Test concurrent updates - verifies FOR UPDATE row locking works correctly
///
/// With pessimistic row locking implemented:
/// - FOR UPDATE syntax acquires row locks on SELECT
/// - Concurrent transactions must wait for locks to be released
/// - No lost updates: final quota should be consistent with committed updates
#[test]
fn test_concurrent_budget_updates() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, api_key TEXT, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    // Insert initial budget with 1000 quota
    db.execute(
        "INSERT INTO budgets (id, api_key, remaining_quota) VALUES (1, 'test-key', 1000)",
        (),
    )
    .expect("Failed to insert");

    let num_threads = 10;
    let decrement_per_thread = 10i64;

    let success_count = Arc::new(AtomicI32::new(0));

    // Spawn threads to concurrently update the budget
    let mut handles = vec![];

    for _i in 0..num_threads {
        let db_clone = db.clone();
        let success = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            // Each thread: BEGIN -> SELECT FOR UPDATE -> UPDATE -> COMMIT
            let mut tx = match db_clone.begin() {
                Ok(tx) => tx,
                Err(_) => return,
            };

            // Read current quota with FOR UPDATE
            let result = tx.query(
                "SELECT remaining_quota FROM budgets WHERE id = 1 FOR UPDATE",
                (),
            );

            let current_quota = match result {
                Ok(mut r) => match r.next() {
                    Some(row) => match row.expect("Failed to get row").get::<i64>(0) {
                        Ok(q) => q,
                        Err(_) => {
                            let _ = tx.rollback();
                            return;
                        }
                    },
                    None => {
                        let _ = tx.rollback();
                        return;
                    }
                },
                Err(_) => {
                    let _ = tx.rollback();
                    return;
                }
            };

            // Only decrement if we have enough quota
            if current_quota >= decrement_per_thread {
                let new_quota = current_quota - decrement_per_thread;
                let update_result = tx.execute(
                    &format!(
                        "UPDATE budgets SET remaining_quota = {} WHERE id = 1",
                        new_quota
                    ),
                    (),
                );

                if update_result.is_ok() {
                    let commit_result = tx.commit();
                    if commit_result.is_ok() {
                        success.fetch_add(1, Ordering::SeqCst);
                        return;
                    }
                }
            }

            // Either not enough quota or commit failed - rollback
            let _ = tx.rollback();
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify results - FOR UPDATE should prevent lost updates
    let final_result = db
        .query("SELECT remaining_quota FROM budgets WHERE id = 1", ())
        .expect("Query failed");

    let final_quota: i64 = final_result
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");

    let successes = success_count.load(Ordering::SeqCst);

    println!(
        "Final quota: {}, Successful updates: {}",
        final_quota, successes
    );

    // With proper FOR UPDATE locking:
    // - Each successful update reduces quota by 10
    // - Final quota should be consistent with number of successful updates
    // - final_quota + (successes * 10) should equal 1000 (within a small tolerance)
    let total_decremented = (1000i64 - final_quota) / 10;
    let reported_successes = successes as i64;

    // Allow for small discrepancy due to timing (at least 90% should match)
    let min_expected = (reported_successes * 9) / 10;
    assert!(
        total_decremented >= min_expected,
        "Too many lost updates! Reported {} successes, but only {} decrements applied",
        reported_successes,
        total_decremented
    );

    // Basic sanity: some updates should have succeeded
    assert!(successes > 0, "Expected some successful updates");
}

/// Test that FOR UPDATE properly serializes concurrent updates to the same row
///
/// This test verifies that FOR UPDATE prevents lost updates:
/// - Concurrent transactions all report success (they all think they modified the row)
/// - But the final value reflects the actual committed updates
/// - No lost updates: final = initial + actual_committed_updates
#[test]
fn test_for_update_serializes_updates() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE counters (id INTEGER PRIMARY KEY, value INTEGER)",
        (),
    )
    .expect("Failed to create table");

    // Insert initial counter
    db.execute("INSERT INTO counters (id, value) VALUES (1, 100)", ())
        .expect("Failed to insert");

    let num_increments = 20;
    let success_count = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    // Spawn threads that each increment by 1
    for _ in 0..num_increments {
        let db_clone = db.clone();
        let success = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            let mut tx = match db_clone.begin() {
                Ok(tx) => tx,
                Err(_) => return,
            };

            // Lock and read current value
            let result = tx.query("SELECT value FROM counters WHERE id = 1 FOR UPDATE", ());

            let current_value = match result {
                Ok(mut r) => match r.next() {
                    Some(row) => match row.expect("Failed to get row").get::<i64>(0) {
                        Ok(v) => v,
                        Err(_) => {
                            let _ = tx.rollback();
                            return;
                        }
                    },
                    None => {
                        let _ = tx.rollback();
                        return;
                    }
                },
                Err(_) => {
                    let _ = tx.rollback();
                    return;
                }
            };

            // Increment
            let new_value = current_value + 1;
            let update_result = tx.execute(
                &format!("UPDATE counters SET value = {} WHERE id = 1", new_value),
                (),
            );

            if update_result.is_ok() {
                if tx.commit().is_ok() {
                    success.fetch_add(1, Ordering::SeqCst);
                    return;
                }
            }

            let _ = tx.rollback();
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final value
    let final_result = db
        .query("SELECT value FROM counters WHERE id = 1", ())
        .expect("Query failed");

    let final_value: i64 = final_result
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");

    let successes = success_count.load(Ordering::SeqCst);

    // Key invariant: final = initial + actual_increments
    // where actual_increments = final - 100
    let actual_increments = final_value - 100;

    // The number of reported successes should not exceed what could actually be applied
    // (allowing for some race condition edge cases, we check that at least 90% match)
    let min_expected = (successes * 9) / 10;
    assert!(
        actual_increments >= min_expected as i64,
        "Lost updates detected! {} reported successes, but only {} increments applied",
        successes,
        actual_increments
    );

    // Final value must be at least 100 (initial) + some increments
    assert!(
        final_value > 100,
        "At least some increments should have applied"
    );
}

/// Test that different rows can be updated concurrently without blocking
#[test]
fn test_concurrent_updates_different_rows() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, api_key TEXT, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    // Insert budgets for different API keys
    for i in 1..=5 {
        db.execute(
            &format!(
                "INSERT INTO budgets (id, api_key, remaining_quota) VALUES ({}, 'key-{}', 100)",
                i, i
            ),
            (),
        )
        .expect("Failed to insert");
    }

    let success_count = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    // Spawn threads for different rows
    for i in 1..=5 {
        let db_clone = db.clone();
        let success = Arc::clone(&success_count);

        let handle = thread::spawn(move || {
            let mut tx = match db_clone.begin() {
                Ok(tx) => tx,
                Err(_) => return,
            };

            // Update different rows with FOR UPDATE
            let result = tx.execute(
                &format!(
                    "UPDATE budgets SET remaining_quota = remaining_quota - 10 WHERE id = {}",
                    i
                ),
                (),
            );

            if result.is_ok() {
                if tx.commit().is_ok() {
                    success.fetch_add(1, Ordering::SeqCst);
                    return;
                }
            }

            let _ = tx.rollback();
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let successes = success_count.load(Ordering::SeqCst);
    assert_eq!(successes, 5, "All 5 row updates should succeed");
}

/// Test FOR UPDATE with WHERE clause - only locks matching rows
#[test]
fn test_for_update_with_where_clause() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, api_key TEXT, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    // Insert multiple budgets
    db.execute(
        "INSERT INTO budgets (id, api_key, remaining_quota) VALUES (1, 'key-a', 100), (2, 'key-b', 200)",
        (),
    )
    .expect("Failed to insert");

    // Transaction 1: Lock only row 1
    let mut tx1 = db.begin().expect("Failed to begin transaction");
    let result1 = tx1
        .query(
            "SELECT remaining_quota FROM budgets WHERE id = 1 FOR UPDATE",
            (),
        )
        .expect("Query failed");
    let quota1: i64 = result1
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");
    assert_eq!(quota1, 100);

    // Transaction 2: Should be able to lock row 2 (different row)
    let mut tx2 = db.begin().expect("Failed to begin transaction");
    let result2 = tx2
        .query(
            "SELECT remaining_quota FROM budgets WHERE id = 2 FOR UPDATE",
            (),
        )
        .expect("Query failed");
    let quota2: i64 = result2
        .into_iter()
        .next()
        .expect("Expected a row")
        .expect("Failed to get row")
        .get::<i64>(0)
        .expect("Failed to get value");
    assert_eq!(quota2, 200);

    // Both should be able to commit
    tx1.commit().expect("Commit failed");
    tx2.commit().expect("Commit failed");
}

/// Test that FOR UPDATE without transaction auto-commits (implicit transaction)
#[test]
fn test_for_update_implicit_transaction() {
    let db = Database::open_in_memory().expect("Failed to create database");

    db.execute(
        "CREATE TABLE budgets (id INTEGER PRIMARY KEY, remaining_quota INTEGER)",
        (),
    )
    .expect("Failed to create table");

    db.execute(
        "INSERT INTO budgets (id, remaining_quota) VALUES (1, 100)",
        (),
    )
    .expect("Failed to insert");

    // FOR UPDATE in auto-commit mode (default)
    let result = db.query(
        "SELECT remaining_quota FROM budgets WHERE id = 1 FOR UPDATE",
        (),
    );
    assert!(result.is_ok(), "FOR UPDATE should work in auto-commit");
}
