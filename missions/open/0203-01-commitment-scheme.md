# Mission: Pedersen Commitment Scheme

## Status
Open

## RFC
RFC-0203: Confidential Query Operations

## Acceptance Criteria
- [ ] Implement `pedersen_commit()` function
- [ ] Implement `open_commitment()` function
- [ ] Define `Commitment` type alias
- [ ] Add batch commitment function
- [ ] Add tests for commitment binding property
- [ ] Add tests for commitment hiding property

## Dependencies
- RFC-0201 (STWO Integration) - Complete

## Enables
- Mission 0203-02 (Confidential Query Types)

## Implementation Notes

**Files to Create:**
- `src/zk/commitment.rs` - Commitment scheme

**Types and Functions:**
```rust
pub type Commitment = [u8; 32];

pub fn pedersen_commit(value: i64, randomness: u64) -> Commitment {
    // C = g^value * h^randomness
    let g = GENERATOR_G;
    let h = GENERATOR_H;
    let point = g.mul(value).add(h.mul(randomness));
    point.to_bytes()
}

pub fn open_commitment(
    commitment: &Commitment,
    value: i64,
    randomness: u64,
) -> bool {
    pedersen_commit(value, randomness) == *commitment
}

pub fn pedersen_commit_batch(values: &[i64]) -> Vec<Commitment> {
    values.iter()
        .map(|&v| pedersen_commit(v, thread_rng().gen()))
        .collect()
}
```

**Constants:**
```rust
const GENERATOR_G: Point = /* ... */;
const GENERATOR_H: Point = /* ... */;
```

**Tests:**
```rust
#[test]
fn test_commitment_binding() {
    // Can't open commitment to different value
}

#[test]
fn test_commitment_hiding() {
    // Same value commits to different outputs
}
```

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
