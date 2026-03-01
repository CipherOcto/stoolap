# Mission: Compressed Proof Verification

## Status
Open

## RFC
RFC-0202: Compressed Proof Format for Batch Verification

## Acceptance Criteria
- [ ] Implement `CompressedProof::verify()` method
- [ ] Verify STARK proof via STWOVerifier
- [ ] Check program hash against registry
- [ ] Add tests for valid proof verification
- [ ] Add tests for invalid proof rejection
- [ ] Benchmark verification time (target: <100ms)

## Dependencies
- Mission 0201-05 (Prover Interface)
- Mission 0202-01 (Compressed Proof Types)
- Mission 0202-02 (Proof Generation)

## Enables
- RFC-0202 completion

## Implementation Notes

**Files to Modify:**
- `src/zk/compressed.rs` - Add verification method

**Implementation:**
```rust
impl CompressedProof {
    pub fn verify(&self) -> bool {
        // 1. Get program from registry
        let registry = CairoProgramRegistry::get_global();
        let program = match registry.get(&self.program_hash) {
            Some(p) => p,
            None => return false,
        };

        // 2. Verify STARK proof
        let verifier = STWOVerifier::new();
        match verifier.verify(&self.stark_proof) {
            Ok(true) => true,
            _ => false,
        }
    }
}
```

**Tests:**
```rust
#[test]
fn test_verify_valid_compressed_proof() {
    // Generate compressed proof
    // Verify returns true
}

#[test]
fn test_verify_invalid_proof() {
    // Tamper with proof
    // Verify returns false
}

#[test]
fn test_verify_unauthorized_program() {
    // Use program not in allowlist
    // Verify returns false
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
