//! Reproduction test for the WhatsApp adapter's StoolapStore::save
//! INSERT failure: DELETE+INSERT same primary key in one transaction.

use stoolap::Database;

#[test]
fn delete_then_insert_same_pk_in_transaction() {
    let db = Database::open("memory://test_pk").unwrap();
    db.execute(
        "CREATE TABLE device (id INTEGER PRIMARY KEY, name TEXT NOT NULL, counter INTEGER NOT NULL)",
        (),
    ).unwrap();

    db.execute::<(_, _, _)>(
        "INSERT INTO device (id, name, counter) VALUES (?, ?, ?)",
        (1i64, "initial", 0i64),
    )
    .unwrap();

    // Transaction: DELETE + INSERT with same PK.
    let mut tx = db.begin().unwrap();
    tx.execute("DELETE FROM device WHERE id = 1", ()).unwrap();
    let result = tx.execute::<(_, _, _)>(
        "INSERT INTO device (id, name, counter) VALUES (?, ?, ?)",
        (1i64, "updated", 1i64),
    );
    assert!(
        result.is_ok(),
        "INSERT after DELETE same PK should succeed: {:?}",
        result.err()
    );
    tx.commit().unwrap();

    let mut rows = db
        .query("SELECT name FROM device WHERE id = 1", ())
        .unwrap();
    let row = rows.next().unwrap().unwrap();
    let name: String = row.get(0).unwrap();
    assert_eq!(name, "updated");
}

#[test]
fn concurrent_delete_insert_two_handles() {
    let db1 = Database::open("memory://test_concurrent").unwrap();
    db1.execute(
        "CREATE TABLE device (id INTEGER PRIMARY KEY, name TEXT NOT NULL, counter INTEGER NOT NULL)",
        (),
    ).unwrap();
    db1.execute::<(_, _, _)>(
        "INSERT INTO device (id, name, counter) VALUES (?, ?, ?)",
        (1i64, "init", 0i64),
    )
    .unwrap();

    let db2 = db1.clone();

    for i in 0..5 {
        {
            let mut tx = db1.begin().unwrap();
            tx.execute("DELETE FROM device WHERE id = 1", ()).unwrap();
            let name = format!("main_{i}");
            let r = tx.execute(
                "INSERT INTO device (id, name, counter) VALUES (1, ?, ?)",
                (name.as_str(), i as i64),
            );
            assert!(r.is_ok(), "main INSERT failed at {i}: {:?}", r.err());
            tx.commit().unwrap();
        }
        {
            let mut tx = db2.begin().unwrap();
            tx.execute("DELETE FROM device WHERE id = 1", ()).unwrap();
            let name = format!("bg_{i}");
            let r = tx.execute(
                "INSERT INTO device (id, name, counter) VALUES (1, ?, ?)",
                (name.as_str(), i as i64 + 100),
            );
            assert!(r.is_ok(), "bg INSERT failed at {i}: {:?}", r.err());
            tx.commit().unwrap();
        }
    }

    let mut rows = db1
        .query("SELECT name FROM device WHERE id = 1", ())
        .unwrap();
    assert!(
        rows.next().is_some(),
        "row must exist after concurrent DELETE+INSERT"
    );
}
