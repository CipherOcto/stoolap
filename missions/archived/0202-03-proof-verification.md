# Mission: Compressed Proof Verification

## Status
Completed

## RFC
RFC-0202: Compressed Proof Format for Batch Verification
RFC-0205: STWO Verification Plugin Architecture

## Acceptance Criteria
- [x] Create stwo-plugin crate with C-compatible verification
- [x] Implement `CompressedProof::verify()` using plugin
- [x] Add plugin discovery (env var + default path)
- [x] Add proper error handling (PluginNotFound, PluginError)
- [x] Verify STARK proof via STWO when plugin available
- [x] Check program hash against registry
- [x] Add tests for valid proof verification
- [x] Add tests for invalid proof rejection
- [x] Add tests for missing plugin scenario
- [x] Benchmark verification time (target: <100ms)

## Dependencies
- Mission 0201-05 (Prover Interface)
- Mission 0202-01 (Compressed Proof Types)
- Mission 0202-02 (Proof Generation)

## Enables
- RFC-0202 completion
- RFC-0205 completion

## Implementation Notes

### Files Created

**stwo-plugin/ (new crate outside workspace)**
- `stwo-plugin/Cargo.toml` - cdylib + rlib crate types
- `stwo-plugin/rust-toolchain.toml` - nightly-2025-06-23
- `stwo-plugin/src/lib.rs` - C-compatible exports
- `stwo-plugin/src/verify.rs` - STWO verification wrapper

### Files Modified

- `src/zk/compressed.rs` - Add verify() using plugin
- `src/zk/plugin.rs` - NEW: Plugin loading + error types
- `src/zk/mod.rs` - Export plugin module
- `Cargo.toml` - Add libloading dependency

### Implementation

```rust
// src/zk/plugin.rs - Plugin loading

use libloading::{Library, Symbol};
use std::path::Path;

pub struct STWOPlugin {
    lib: Library,
}

impl STWOPlugin {
    pub fn load(path: &Path) -> Result<Self, PluginError> { ... }
    pub fn verify(&self, proof: &[u8]) -> Result<bool, PluginError> { ... }
}

pub fn load_plugin() -> Result<STWOPlugin, PluginError> { ... }
```

```rust
// src/zk/compressed.rs - verify()

pub fn verify(&self) -> Result<bool, CompressedProofError> {
    self.validate()?;
    let plugin = crate::zk::plugin::load_plugin()
        .map_err(CompressedProofError::from)?;
    plugin.verify(&self.stark_proof.proof)
        .map_err(CompressedProofError::from)
}
```

### Plugin Interface

```rust
// stwo-plugin/src/lib.rs

#[no_mangle]
pub unsafe extern "C" fn stark_verify_proof(
    proof_bytes: *const u8,
    proof_len: usize,
) -> StarkVerifyResult { ... }

#[no_mangle]
pub extern "C" fn stark_plugin_version() -> *const c_char { ... }
```

### Discovery

1. Environment variable: `STOOLAP_STWO_PLUGIN`
2. Default path: `../stwo-plugin/target/release/libstwo_plugin.so`
3. Error: Clear message with setup instructions

### Benchmark Results

| Benchmark | Median | Target |
|-----------|--------|--------|
| stark_real_proof_verification_merkle_batch | 15.7ms | <100ms |

## Claimant
Claude Agent

## Pull Request
#105 (merged)

## Commits
- 908a63e - feat: Implement STWO plugin architecture for verification
- 105fa06 - feat: Complete STWO plugin verification implementation

## Completion Date
2026-03-02
