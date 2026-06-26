/// Reproduces the exact WhatsApp adapter error: DELETE+INSERT in a transaction.
/// Two cases:
/// 1. UNIQUE constraint (most WhatsApp tables: sessions, identities, etc.)
/// 2. Inline PRIMARY KEY (device table)
use stoolap::Database;

#[test]
fn wa_unique_delete_insert_sessions() {
    let db = Database::open("memory://wa_sessions").unwrap();
    db.execute(
        "CREATE TABLE sessions (address TEXT NOT NULL, record BLOB NOT NULL, device_id INTEGER NOT NULL, UNIQUE (address, device_id))",
        (),
    ).unwrap();

    db.execute(
        "INSERT INTO sessions (address, record, device_id) VALUES (?, ?, ?)",
        ("addr1", vec![1u8, 2, 3].as_slice(), 1i64),
    )
    .unwrap();

    // Transaction: DELETE + INSERT same unique key
    let mut tx = db.begin().unwrap();
    tx.execute(
        "DELETE FROM sessions WHERE address = ? AND device_id = ?",
        ("addr1", 1i64),
    )
    .unwrap();
    let result = tx.execute(
        "INSERT INTO sessions (address, record, device_id) VALUES (?, ?, ?)",
        ("addr1", vec![4u8, 5, 6].as_slice(), 1i64),
    );
    assert!(
        result.is_ok(),
        "INSERT after DELETE same UNIQUE key should succeed: {:?}",
        result.err()
    );
    tx.commit().unwrap();

    let mut rows = db
        .query(
            "SELECT record FROM sessions WHERE address = ? AND device_id = ?",
            ("addr1", 1i64),
        )
        .unwrap();
    let row = rows.next().unwrap().unwrap();
    let record: Vec<u8> = row.get(0).unwrap();
    assert_eq!(record, vec![4, 5, 6]);
}

#[test]
fn wa_pk_delete_insert_device() {
    let db = Database::open("memory://wa_device").unwrap();
    db.execute(
        "CREATE TABLE device (id INTEGER PRIMARY KEY, pn TEXT NOT NULL, noise_key BLOB NOT NULL)",
        (),
    )
    .unwrap();

    db.execute(
        "INSERT INTO device (id, pn, noise_key) VALUES (?, ?, ?)",
        (1i64, "5511999", vec![10u8, 20, 30].as_slice()),
    )
    .unwrap();

    // Transaction: DELETE + INSERT same PK (matches StoolapStore save_device)
    let mut tx = db.begin().unwrap();
    tx.execute("DELETE FROM device WHERE id = ?", (1i64,))
        .unwrap();
    let result = tx.execute(
        "INSERT INTO device (id, pn, noise_key) VALUES (?, ?, ?)",
        (1i64, "5511888", vec![40u8, 50, 60].as_slice()),
    );
    assert!(
        result.is_ok(),
        "INSERT after DELETE same PK should succeed: {:?}",
        result.err()
    );
    tx.commit().unwrap();

    let mut rows = db
        .query("SELECT pn FROM device WHERE id = ?", (1i64,))
        .unwrap();
    let row = rows.next().unwrap().unwrap();
    let pn: String = row.get(0).unwrap();
    assert_eq!(pn, "5511888");
}

#[test]
fn wa_repeated_save_cycles() {
    // WhatsApp calls save() many times (each is DELETE+INSERT in a txn)
    let db = Database::open("memory://wa_cycles").unwrap();
    db.execute(
        "CREATE TABLE sessions (address TEXT NOT NULL, record BLOB NOT NULL, device_id INTEGER NOT NULL, UNIQUE (address, device_id))",
        (),
    ).unwrap();

    db.execute(
        "INSERT INTO sessions (address, record, device_id) VALUES (?, ?, ?)",
        ("addr1", vec![0u8].as_slice(), 1i64),
    )
    .unwrap();

    for i in 0..10 {
        let mut tx = db.begin().unwrap();
        tx.execute(
            "DELETE FROM sessions WHERE address = ? AND device_id = ?",
            ("addr1", 1i64),
        )
        .unwrap();
        let record = vec![i as u8; 4];
        let r = tx.execute(
            "INSERT INTO sessions (address, record, device_id) VALUES (?, ?, ?)",
            ("addr1", record.as_slice(), 1i64),
        );
        assert!(r.is_ok(), "cycle {i}: INSERT failed: {:?}", r.err());
        tx.commit().unwrap();
    }

    let mut rows = db
        .query(
            "SELECT record FROM sessions WHERE address = ? AND device_id = ?",
            ("addr1", 1i64),
        )
        .unwrap();
    let row = rows.next().unwrap().unwrap();
    let record: Vec<u8> = row.get(0).unwrap();
    assert_eq!(record, vec![9u8; 4], "final cycle should have last value");
}
