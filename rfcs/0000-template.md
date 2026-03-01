# RFC-XXXX: [Title]

## Status
Draft | Accepted | Replaced | Deprecated

## Summary
One-paragraph overview of what this RFC specifies.

## Motivation
Why is this RFC needed? What problem does it solve?

## Specification

### Data Structures
(If applicable)

```rust
pub struct Example {
    pub field: Type,
}
```

### Functions/Methods
(If applicable)

```rust
pub fn example_function() -> ReturnType {
    // Implementation
}
```

### Encoding Format
(If applicable)

| Field | Size | Description |
|-------|------|-------------|
| field1 | 4 bytes | Description |

### Algorithms
(If applicable)

1. Step 1
2. Step 2
3. Step 3

## Rationale

### Why This Approach?

Explain why this specific approach was chosen over alternatives.

### Alternatives Considered

| Alternative | Pros | Cons | Chosen? |
|------------|------|------|---------|
| Option A | Pro 1, Pro 2 | Con 1 | ❌ |
| Option B | Pro 1 | Con 1, Con 2 | ✅ |

## Implementation

### Phased Approach

1. **Phase 1:** Component A
2. **Phase 2:** Component B
3. **Phase 3:** Integration

### Dependencies

- Requires: RFC-XXXX
- Enables: RFC-YYYY

### Testing Requirements

- Unit tests for all functions
- Integration tests for interactions
- Benchmarks for performance-critical code

## Performance Considerations

- Time complexity: O(n) where n is...
- Space complexity: O(m) where m is...
- Expected performance: <X ms for typical case

## Security Considerations

- Cryptographic requirements: SHA-256, etc.
- Attack vectors: Consider potential attacks
- Mitigation strategies: How to prevent attacks

## Backward Compatibility

- Breaking changes: List any breaking changes
- Migration path: How to migrate from previous version
- Deprecation timeline: When old version is removed

## Related Use Cases

- [Use Case Name](../docs/use-cases/filename.md) - Description

## Related RFCs

- [RFC-XXXX](./xxxx-title.md) - Related RFC
- [RFC-YYYY](./yyyy-title.md) - Related RFC

## Open Questions

1. Question 1?
   - Possible Answer A
   - Possible Answer B
   - **Resolution:** TBD

## References

- [Paper/Resource](URL) - Description
- [Similar System](URL) - Comparison
