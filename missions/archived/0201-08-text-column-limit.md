# Mission: TEXT Column Limit Enforcement (1 MiB)

## Status
Pending

## RFC
RFC-0201: Binary BLOB Type Support

## Context
RFC-0201 specifies BYTEA with a 16 MiB maximum length. During implementation, TEXT columns were identified as lacking a corresponding length limit. TEXT columns should enforce a 1 MiB (1,048,576 byte) maximum to prevent unbounded storage allocation and ensure predictable memory usage.

## Acceptance Criteria
- [ ] Add `max_text_length: Option<u32>` field to `SchemaColumn` (parallel to `blob_length`)
- [ ] Update `SchemaColumn::new()` and `SchemaColumn::with_constraints()` to accept `max_text_length`
- [ ] Update DDL parser to parse `TEXT(N)` and `VARCHAR(N)` length constraints
- [ ] Update `validate_schema()` to enforce max_text_length bounds (non-zero, max 1 MiB)
- [ ] Add unit tests for TEXT length validation
- [ ] Update documentation

## Dependencies
- Mission 0201-01 through 0201-07 (BYTEA Core implementation) — Complete

## Files to Modify
- `src/core/schema.rs` — Add `max_text_length` field and validation
- `src/executor/ddl.rs` — Parse TEXT/VARCHAR length constraint
- `src/executor/utils.rs` — Parse optional length

## Tech Notes
- TEXT limit: 1,048,576 bytes (1 MiB)
- TEXT is distinct from VARCHAR — both map to `DataType::Text`
- Existing TEXT columns with `max_text_length: None` should remain valid (unbounded, for migration compatibility)

## Claimant
TBD

## Pull Request
TBD
