# RFC-0201 BYTEA Core Blob Type — Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `DataType::Blob`, `Value::Blob`, serialization (wire tag 12), DDL parsing, and schema validation for BYTEA columns in stoolap.

**Architecture:** Add `Blob = 10` to `DataType` enum, add `Value::Blob(CompactArc<[u8]>)` as a first-class variant (not Extension), wire format `[u8:12][u32_be:len][u8..len:data]`. BYTEA columns are rejected at schema validation with "null bitmap integration required" until null bitmap is implemented.

**Tech Stack:** Rust, stoolap core types, `octo-determin` crate (for CompactArc).

---

## Task 1: Add `DataType::Blob = 10` to types.rs

**Files:**
- Modify: `src/core/types.rs:27-64` (DataType enum)
- Modify: `src/core/types.rs:87-101` (from_u8)
- Modify: `src/core/types.rs:104-118` (Display)
- Modify: `src/core/types.rs:121-146` (FromStr)

**Step 1: Write the failing test**

```rust
// In src/core/tests/ directory (create if not exists)
// tests/data_type_parsing_test.rs
#[test]
fn test_datatype_blob_from_str() {
    use crate::core::types::DataType;
    assert!(matches!("BYTEA".parse::<DataType>(), Ok(DataType::Blob)));
    assert!(matches!("BLOB".parse::<DataType>(), Ok(DataType::Blob)));
    assert!(matches!("BINARY".parse::<DataType>(), Ok(DataType::Blob)));
    assert!(matches!("VARBINARY".parse::<DataType>(), Ok(DataType::Blob)));
}

#[test]
fn test_datatype_blob_display() {
    use crate::core::types::DataType;
    assert_eq!(DataType::Blob.to_string(), "BYTEA");
}

#[test]
fn test_datatype_blob_as_u8() {
    use crate::core::types::DataType;
    assert_eq!(DataType::Blob.as_u8(), 10);
}

#[test]
fn test_datatype_blob_from_u8() {
    use crate::core::types::DataType;
    assert_eq!(DataType::from_u8(10), Some(DataType::Blob));
    assert_eq!(DataType::from_u8(11), None); // 11 is not yet used
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_datatype_blob --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "variant does not exist"

**Step 3: Add Blob to DataType enum**

```rust
// src/core/types.rs — add after Quant = 9,
/// Binary large object for cryptographic hashes and binary data
Blob = 10,
```

**Step 4: Update from_u8 match**

```rust
// src/core/types.rs:from_u8 — add before _ => None
10 => Some(DataType::Blob),
```

**Step 5: Update Display match**

```rust
// src/core/types.rs:Display — add
DataType::Blob => write!(f, "BYTEA"),
```

**Step 6: Update FromStr match**

```rust
// src/core/types.rs:FromStr — add to match
"BYTEA" | "BLOB" | "BINARY" | "VARBINARY" => Ok(DataType::Blob),
```

**Step 7: Update is_orderable**

```rust
// src/core/types.rs:is_orderable — Blob IS orderable (remove from exclusion if present)
pub fn is_orderable(&self) -> bool {
    !matches!(self, DataType::Json | DataType::Vector)
}
```

**Step 8: Run test to verify it passes**

Run: `cargo test test_datatype_blob --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 9: Commit**

```bash
git add src/core/types.rs src/core/tests/
git commit -m "feat(core): add DataType::Blob = 10 with FromStr parsing"
```

---

## Task 2: Add `Value::Blob(CompactArc<[u8]>)` to value.rs

**Files:**
- Modify: `src/core/value.rs:75-100` (Value enum)
- Modify: `src/core/value.rs:105+` (constructors section)
- Modify: `src/core/value.rs` (as_blob, PartialEq, Ord, Hash, compare integration)

**Step 1: Write the failing test**

```rust
// In src/core/tests/ value_tests.rs
#[test]
fn test_value_blob_constructors() {
    use crate::core::value::Value;
    let data = b"hello world";
    let blob = Value::blob(data);
    assert!(blob.as_blob().is_some());
    assert_eq!(blob.as_blob().unwrap(), b"hello world");
}

#[test]
fn test_value_blob_from_vec() {
    use crate::core::value::Value;
    let data = vec![0u8, 1, 2, 3];
    let blob = Value::blob_from_vec(data.clone());
    assert_eq!(blob.as_blob().unwrap(), &data);
}

#[test]
fn test_value_blob_partial_eq() {
    use crate::core::value::Value;
    let a = Value::blob(b"hello");
    let b = Value::blob(b"hello");
    let c = Value::blob(b"world");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_value_blob_hash() {
    use crate::core::value::Value;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    let a = Value::blob(b"hello");
    let b = Value::blob(b"hello");
    let mut ha = DefaultHasher::new();
    let mut hb = DefaultHasher::new();
    a.hash(&mut ha);
    b.hash(&mut hb);
    assert_eq!(ha.finish(), hb.finish());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_value_blob --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "variant does not exist" / "no method named `blob`"

**Step 3: Add BlobOrdering enum**

Add before the Value enum or near the top of value.rs:

```rust
/// Ordering for blob comparison (per RFC-0201)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlobOrdering {
    Less,
    Equal,
    Greater,
}

/// Compare two blobs byte-by-byte in deterministic order
///
/// Algorithm:
/// 1. Compare bytes in ascending index order until difference found
/// 2. If all compared bytes are equal, compare lengths (shorter = less)
///
/// Determinism: This ordering is canonical and reproducible.
fn compare_blob(a: &[u8], b: &[u8]) -> BlobOrdering {
    use std::cmp::Ordering;
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        match a[i].cmp(&b[i]) {
            Ordering::Less => return BlobOrdering::Less,
            Ordering::Greater => return BlobOrdering::Greater,
            Ordering::Equal => continue,
        }
    }
    match a.len().cmp(&b.len()) {
        Ordering::Less => BlobOrdering::Less,
        Ordering::Greater => BlobOrdering::Greater,
        Ordering::Equal => BlobOrdering::Equal,
    }
}
```

**Step 4: Add Blob variant to Value enum**

```rust
// Add after Timestamp variant:
/// Binary large object — stored as CompactArc<[u8]> for zero-copy sharing.
/// INVARIANT: The Arc is always heap-allocated; there is no inline/blob case.
Blob(CompactArc<[u8]>),
```

Also update the module comment about Extension being for "Future types" — the comment at line 98 says "Future types (Blob, Array, etc.)" — remove Blob from that list.

**Step 5: Add blob constructors to Value impl block**

```rust
/// Create a Blob from a byte slice (copies into CompactArc)
pub fn blob(data: &[u8]) -> Self {
    Value::Blob(CompactArc::from(data))
}

/// Create a Blob from an owned Vec (no copy — takes ownership of Arc)
pub fn blob_from_vec(data: Vec<u8>) -> Self {
    Value::Blob(CompactArc::from(data))
}

/// Create a Blob from a CompactArc (zero-copy)
pub fn blob_from_arc(data: CompactArc<[u8]>) -> Self {
    Value::Blob(data)
}

/// Extract blob data as byte slice
pub fn as_blob(&self) -> Option<&[u8]> {
    match self {
        Value::Blob(data) => Some(data),
        _ => None,
    }
}

/// Extract blob data as a slice and its length
pub fn as_blob_len(&self) -> Option<(&[u8], usize)> {
    match self {
        Value::Blob(data) => Some((data, data.len())),
        _ => None,
    }
}

/// Extract blob data as a 32-byte array (for SHA256 key_hash columns)
/// Returns None if the blob is not exactly 32 bytes.
pub fn as_blob_32(&self) -> Option<[u8; 32]> {
    match self {
        Value::Blob(data) if data.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(data);
            Some(arr)
        }
        _ => None,
    }
}
```

**Step 6: Add PartialEq for Value::Blob**

Find the PartialEq impl for Value and add:
```rust
(Value::Blob(a), Value::Blob(b)) => a == b,
```

**Step 7: Add Blob to type_discriminant function in Ord impl**

Find the `type_discriminant` inner function in `impl Ord for Value` and add:
```rust
Value::Blob(_) => 10,
```

**Step 8: Add Ord arm for Value::Blob**

In the same `Ord` impl's same-type comparison block (after Extension case):
```rust
(Value::Blob(_), Value::Blob(_)) => Ordering::Equal,
// Blob uses discriminant ordering only — per RFC-0201.
// For SQL ORDER BY: use compare_blob via Value::compare.
```

**Step 9: Add Hash for Value::Blob**

Find the Hash impl and add:
```rust
Value::Blob(data) => {
    let mut hasher = state;
    hasher.write_u8(10); // discriminant
    hasher.write(data);
}
```

**Step 10: Run test to verify it passes**

Run: `cargo test test_value_blob --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 11: Commit**

```bash
git add src/core/value.rs
git commit -m "feat(core): add Value::Blob(CompactArc<[u8]>) with constructors and accessors"
```

---

## Task 3: Add blob_length to SchemaColumn

**Files:**
- Modify: `src/core/schema.rs:29-67` (SchemaColumn struct)

**Step 1: Write the failing test**

```rust
#[test]
fn test_schema_column_blob_length() {
    use crate::core::schema::SchemaColumn;
    use crate::core::types::DataType;
    let col = SchemaColumn::simple(0, "key_hash", DataType::Blob)
        .with_blob_length(32);
    assert_eq!(col.blob_length, Some(32));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_schema_column_blob_length --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "field does not exist"

**Step 3: Add blob_length field to SchemaColumn struct**

```rust
/// Decimal scale for DQA/Quant columns (0-18, 0 = not a quant column)
pub quant_scale: u8,

/// Fixed length for BLOB columns (None = variable length)
pub blob_length: Option<u32>,
```

**Step 4: Initialize blob_length in SchemaColumn::new**

Add to the `new` constructor body initialization:
```rust
quant_scale: 0,
blob_length: None,
```

**Step 5: Initialize blob_length in with_constraints and with_default_value**

Add `blob_length: None` to both constructors.

**Step 6: Add with_blob_length builder method**

```rust
/// Set blob length (for BYTEA(N) columns)
pub fn with_blob_length(mut self, len: u32) -> Self {
    self.blob_length = Some(len);
    self
}
```

**Step 7: Run test to verify it passes**

Run: `cargo test test_schema_column_blob_length --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 8: Commit**

```bash
git add src/core/schema.rs
git commit -m "feat(core): add blob_length field to SchemaColumn"
```

---

## Task 4: Add serialize_blob / deserialize_blob and tag 12 to persistence.rs

**Files:**
- Modify: `src/storage/mvcc/persistence.rs:809-867` (serialize_value)
- Modify: `src/storage/mvcc/persistence.rs:870+` (deserialize_value)

**Step 1: Write the failing test**

```rust
// In src/storage/mvcc/tests/ persistence_blob_test.rs
#[test]
fn test_blob_serialize_roundtrip() {
    use crate::core::value::Value;
    use crate::storage::mvcc::persistence::{serialize_value, deserialize_value};
    let original = Value::blob(b"\x01\x02\x03\x04\x05");
    let serialized = serialize_value(&original).unwrap();
    // Tag 12, then u32_be length (5), then data
    assert_eq!(serialized[0], 12);
    assert_eq!(&serialized[1..5], &5u32.to_be_bytes());
    assert_eq!(&serialized[5..], b"\x01\x02\x03\x04\x05");
    let deserialized = deserialize_value(&serialized).unwrap();
    assert_eq!(deserialized.as_blob(), Some(b"\x01\x02\x03\x04\x05"));
}

#[test]
fn test_blob_empty_roundtrip() {
    use crate::core::value::Value;
    use crate::storage::mvcc::persistence::{serialize_value, deserialize_value};
    let original = Value::blob(b"");
    let serialized = serialize_value(&original).unwrap();
    assert_eq!(serialized[0], 12);
    assert_eq!(&serialized[1..5], &0u32.to_be_bytes());
    let deserialized = deserialize_value(&serialized).unwrap();
    assert_eq!(deserialized.as_blob(), Some(b""));
}

#[test]
fn test_blob_deserialize_truncated() {
    use crate::storage::mvcc::persistence::deserialize_value;
    // Only tag + partial length, no data
    let result = deserialize_value(&[12, 0, 0, 1]); // tag=12, len=256, but no data
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_blob_serialize_roundtrip --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "variant does not exist" or "non-exhaustive patterns"

**Step 3: Add Value::Blob arm to serialize_value**

Find the `match value` in `serialize_value` and add before the closing brace:

```rust
Value::Blob(data) => {
    buf.push(12); // wire tag 12
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(data);
}
```

**Step 4: Add tag 12 arm to deserialize_value**

Find the match on type_tag in `deserialize_value`. Add before the closing brace of the match:

```rust
12 => {
    // Blob — u32_be length prefix, per RFC-0201
    if rest.len() < 4 {
        return Err(Error::internal("missing blob length"));
    }
    let len = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
    if rest.len() < 4 + len {
        return Err(Error::internal("missing blob data"));
    }
    let blob_data = CompactArc::from(&rest[4..4 + len]);
    Ok(Value::Blob(blob_data))
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_blob_serialize_roundtrip --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 6: Commit**

```bash
git add src/storage/mvcc/persistence.rs
git commit -m "feat(storage): add serialize/deserialize for Value::Blob (wire tag 12, BE length)"
```

---

## Task 5: Update DDL parser for BYTEA/BLOB/BINARY/VARBINARY

**Files:**
- Modify: `src/executor/ddl.rs` (find the BLOB keyword mapping)
- Modify: `src/executor/ddl.rs` (add BYTEA rejection for CREATE TABLE)

**Step 1: Write the failing test**

```rust
// In src/executor/tests/ ddl_blob_test.rs
#[test]
fn test_parse_bytea_column() {
    use crate::executor::ddl::*;
    let sql = "CREATE TABLE t (key_hash BYTEA(32))";
    // This should parse — rejection is at schema validation, not parsing
    // For now just verify the type keyword is recognized
    // Full DDL parsing test goes here after parser is updated
}

#[test]
fn test_parse_blob_keyword() {
    use crate::core::types::DataType;
    assert_eq!("BLOB".parse::<DataType>().unwrap(), DataType::Blob);
    assert_eq!("BYTEA".parse::<DataType>().unwrap(), DataType::Blob);
    assert_eq!("BINARY".parse::<DataType>().unwrap(), DataType::Blob);
    assert_eq!("VARBINARY".parse::<DataType>().unwrap(), DataType::Blob);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_parse_blob_keyword --no-default-features -- --nocapture 2>&1`
Expected: PASS (DataType parsing already done in Task 1) — verify it passes

Actually skip to Step 4 — the FromStr is already updated.

**Step 3: Find and update BLOB → Text mapping in DDL parser**

Search for `"BLOB" | "BINARY" | "VARBINARY" => Ok(DataType::Text)` and change to:
```rust
"BYTEA" | "BLOB" | "BINARY" | "VARBINARY" => Ok(DataType::Blob),
```

Also add `"BYTEA"` to the match.

**Step 4: Handle BYTEA(N) length constraint**

Find where `BYTEA` type is parsed and add blob_length extraction. This typically involves a regex or parsing in the column definition parsing section. The plan is: if the type contains `(N)` after BYTEA/BINARY/VARBINARY, extract N and store in `SchemaColumn.blob_length`.

**Step 5: Add BYTEA rejection at CREATE TABLE time**

In the schema validation / CREATE TABLE path, add:
```rust
if column.data_type == DataType::Blob {
    return Err("BYTEA columns not supported: null bitmap integration required".into());
}
```

This goes in the schema building / validation step, NOT in the parser.

**Step 6: Run tests**

Run: `cargo test test_parse_blob --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 7: Commit**

```bash
git add src/executor/ddl.rs
git commit -m "feat(executor): map BYTEA/BLOB/BINARY/VARBINARY to DataType::Blob"
```

---

## Task 6: Add ToParam implementations

**Files:**
- Modify: `src/api/params.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_to_param_blob() {
    use crate::api::params::ToParam;
    let blob_data = vec![0u8, 1, 2, 3];
    let value = blob_data.to_param();
    assert_eq!(value.as_blob(), Some(&blob_data.as_slice()));

    let arr: [u8; 4] = [0, 1, 2, 3];
    let value2 = arr.to_param();
    assert_eq!(value2.as_blob(), Some(&arr.as_slice()));

    let slice: &[u8] = &[0, 1, 2, 3];
    let value3 = slice.to_param();
    assert_eq!(value3.as_blob(), Some(slice));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_to_param_blob --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "no method `to_param`"

**Step 3: Add ToParam impls to params.rs**

```rust
impl ToParam for Vec<u8> {
    fn to_param(&self) -> Value {
        Value::blob(self)
    }
}

impl<const N: usize> ToParam for [u8; N] {
    fn to_param(&self) -> Value {
        Value::blob(self.as_slice())
    }
}

impl ToParam for &[u8] {
    fn to_param(&self) -> Value {
        Value::blob(self)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_to_param_blob --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 5: Commit**

```bash
git add src/api/params.rs
git commit -m "feat(api): add ToParam for Vec<u8>, [u8; N], &[u8]"
```

---

## Task 7: Add validate_schema function

**Files:**
- Modify: `src/storage/mvcc/persistence.rs` (or create `src/storage/schema_validation.rs`)

**Step 1: Write the failing test**

```rust
// In src/storage/mvcc/tests/ schema_validation_test.rs
#[test]
fn test_validate_schema_rejects_non_ascending() {
    use crate::storage::mvcc::persistence::validate_schema;
    use crate::core::types::DataType;
    // field_ids must be strictly ascending
    let bad_schema = vec![
        (2, DataType::Text),
        (1, DataType::Blob),
    ];
    assert!(validate_schema(&bad_schema).is_err());
}

#[test]
fn test_validate_schema_accepts_valid() {
    use crate::storage::mvcc::persistence::validate_schema;
    use crate::core::types::DataType;
    let good_schema = vec![
        (1, DataType::Text),
        (2, DataType::Blob),
        (3, DataType::Integer),
    ];
    assert!(validate_schema(&good_schema).is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_validate_schema --no-default-features -- --nocapture 2>&1`
Expected: FAIL — "function does not exist"

**Step 3: Add validate_schema function**

```rust
/// Validate a schema at registration time (CREATE TABLE).
/// Returns DCS_INVALID_STRUCT equivalent error if field_ids are not strictly ascending.
pub fn validate_schema(schema: &[(u32, DataType)]) -> Result<(), Error> {
    if !schema.windows(2).all(|w| w[0].0 < w[1].0) {
        return Err(Error::internal("invalid schema: field_ids not strictly ascending"));
    }
    // Additional validation (nested Structs, etc.) can be added here
    Ok(())
}
```

This is a minimal version. The full RFC-0201 version also validates nested Structs and Enums recursively.

**Step 4: Run test to verify it passes**

Run: `cargo test test_validate_schema --no-default-features -- --nocapture 2>&1`
Expected: PASS

**Step 5: Commit**

```bash
git add src/storage/mvcc/persistence.rs
git commit -m "feat(storage): add validate_schema for CREATE TABLE time checks"
```

---

## Task 8: Run full test suite

**Step 1: Run cargo clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1
```

Fix any warnings.

**Step 2: Run cargo test**

```bash
cargo test 2>&1 | tail -50
```

All tests should pass, including the new blob tests.

**Step 3: Commit final**

```bash
git add -A && git commit -m "feat: RFC-0201 BYTEA Core Blob Type implementation

- DataType::Blob = 10 with FromStr for BYTEA/BLOB/BINARY/VARBINARY
- Value::Blob(CompactArc<[u8]>), constructors, as_blob accessors
- BlobOrdering and compare_blob for byte-level comparison
- Wire tag 12 serialization/deserialization (BE length prefix)
- SchemaColumn.blob_length for BYTEA(N) constraint
- ToParam for Vec<u8>, [u8; N], &[u8]
- validate_schema for CREATE TABLE field_id validation
- BYTEA columns rejected at schema level (null bitmap pending)
"
```

---

## Reference: File Locations

| File | Change |
|------|--------|
| `src/core/types.rs` | DataType::Blob, from_u8, Display, FromStr, is_orderable |
| `src/core/value.rs` | Value::Blob variant, constructors, as_blob*, PartialEq, Ord, Hash, BlobOrdering |
| `src/core/schema.rs` | SchemaColumn.blob_length field and with_blob_length |
| `src/storage/mvcc/persistence.rs` | serialize/deserialize tag 12, validate_schema |
| `src/executor/ddl.rs` | BYTEA/BLOB/BINARY/VARBINARY → DataType::Blob, BYTEA rejection |
| `src/api/params.rs` | ToParam for Vec<u8>, [u8; N], &[u8] |
