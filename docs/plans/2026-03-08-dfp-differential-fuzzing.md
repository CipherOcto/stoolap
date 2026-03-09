# DFP Differential Fuzzing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement continuous differential fuzzing between our DFP implementation and Berkeley SoftFloat reference, with 10,000 random inputs per operation, discrepancy logging, and CI integration.

**Architecture:** Use softfloat-rs crate as reference implementation. Generate random f64/i64 values including edge cases (subnormals, special values). Compare DFP results against SoftFloat results, log discrepancies, and fail tests on mismatch.

**Tech Stack:** Rust, softfloat-rs crate, proptest (already in dev-dependencies)

---

## Task 1: Add softfloat-rs dependency

**Files:**
- Modify: `/home/mmacedoeu/_w/ai/cipherocto/determin/Cargo.toml`

**Step 1: Add dependency**

```toml
[dev-dependencies]
proptest = "1.4"
softfloat-rs = "0.1"
```

**Step 2: Verify it compiles**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo build --features softfloat-rs 2>&1 || cargo build`
Expected: Should compile (may need to adjust version if crate unavailable)

---

## Task 2: Create fuzz module structure

**Files:**
- Create: `/home/mmacedoeu/_w/ai/cipherocto/determin/src/fuzz.rs`

**Step 1: Write minimal module**

```rust
//! Differential fuzzing against Berkeley SoftFloat reference

use crate::{Dfp, DfpClass};
use softfloat_rs::SoftFloat;

/// Compare DFP operation against SoftFloat reference
pub fn compare_add(a: Dfp, b: Dfp) -> (Dfp, f64, bool) {
    // Our DFP result
    let dfp_result = crate::dfp_add(a, b);

    // SoftFloat reference (convert DFP to f64, add, convert back)
    let soft_result = SoftFloat::from_f64(a.to_f64())
        .add(SoftFloat::from_f64(b.to_f64()))
        .to_f64();

    // Compare (NaN matches NaN)
    let dfp_f64 = dfp_result.to_f64();
    let matches = if dfp_f64.is_nan() && soft_result.is_nan() {
        true
    } else if dfp_f64.is_infinite() && soft_result.is_infinite() {
        // Both infinite with same sign
        dfp_f64.is_sign_positive() == soft_result.is_sign_positive()
    } else {
        // Allow small relative error (different rounding modes)
        let diff = (dfp_f64 - soft_result).abs();
        let max_val = dfp_f64.abs().max(soft_result.abs());
        if max_val == 0.0 {
            diff == 0.0
        } else {
            diff / max_val < 1e-10
        }
    };

    (dfp_result, soft_result, matches)
}
```

**Step 2: Run to verify it compiles**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo build`
Expected: PASS

---

## Task 3: Add fuzz test with 10,000 iterations

**Files:**
- Modify: `/home/mmacedoeu/_w/ai/cipherocto/determin/src/fuzz.rs`

**Step 1: Add fuzz test function**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dfp;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rand::Rng;

    #[test]
    fn test_fuzz_add_10k() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut mismatches = Vec::new();

        for i in 0..10000 {
            // Generate random f64, convert to DFP
            let a_f64: f64 = rng.gen();
            let b_f64: f64 = rng.gen();
            let a = Dfp::from_f64(a_f64);
            let b = Dfp::from_f64(b_f64);

            let (dfp_result, soft_result, matches) = compare_add(a, b);

            if !matches {
                mismatches.push((a_f64, b_f64, dfp_result.to_f64(), soft_result));
            }
        }

        // Log mismatches if any
        if !mismatches.is_empty() {
            eprintln!("Found {} mismatches out of 10000:", mismatches.len());
            for (a, b, dfp, soft) in mismatches.iter().take(10) {
                eprintln!("  {} + {} = DFP: {}, SoftFloat: {}", a, b, dfp, soft);
            }
        }

        assert!(mismatches.is_empty(), "Found {} mismatches", mismatches.len());
    }
}
```

**Step 2: Add rand to Cargo.toml**

```toml
[dev-dependencies]
proptest = "1.4"
rand = "0.8"
```

**Step 3: Run test**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo test fuzz`
Expected: Should run and PASS (may find mismatches to fix)

---

## Task 4: Add all four operations (sub, mul, div)

**Files:**
- Modify: `/home/mmacedoeu/_w/ai/cipherocto/determin/src/fuzz.rs`

**Step 1: Add comparison functions for sub, mul, div**

```rust
pub fn compare_sub(a: Dfp, b: Dfp) -> (Dfp, f64, bool) {
    let dfp_result = crate::dfp_sub(a, b);
    let soft_result = SoftFloat::from_f64(a.to_f64())
        .sub(SoftFloat::from_f64(b.to_f64()))
        .to_f64();
    let matches = compare_f64(dfp_result.to_f64(), soft_result);
    (dfp_result, soft_result, matches)
}

pub fn compare_mul(a: Dfp, b: Dfp) -> (Dfp, f64, bool) {
    let dfp_result = crate::dfp_mul(a, b);
    let soft_result = SoftFloat::from_f64(a.to_f64())
        .mul(SoftFloat::from_f64(b.to_f64()))
        .to_f64();
    let matches = compare_f64(dfp_result.to_f64(), soft_result);
    (dfp_result, soft_result, matches)
}

pub fn compare_div(a: Dfp, b: Dfp) -> (Dfp, f64, bool) {
    let dfp_result = crate::dfp_div(a, b);
    let soft_result = SoftFloat::from_f64(a.to_f64())
        .div(SoftFloat::from_f64(b.to_f64()))
        .to_f64();
    let matches = compare_f64(dfp_result.to_f64(), soft_result);
    (dfp_result, soft_result, matches)
}

fn compare_f64(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() { return true; }
    if a.is_infinite() && b.is_infinite() {
        return a.is_sign_positive() == b.is_sign_positive();
    }
    if a.is_infinite() != b.is_infinite() { return false; }
    if a == 0.0 && b == 0.0 { return true; }
    let diff = (a - b).abs();
    let max_val = a.abs().max(b.abs());
    if max_val == 0.0 { diff == 0.0 } else { diff / max_val < 1e-10 }
}
```

**Step 2: Add test functions for sub, mul, div**

```rust
#[test]
fn test_fuzz_sub_10k() { /* similar pattern */ }

#[test]
fn test_fuzz_mul_10k() { /* similar pattern */ }

#[test]
fn test_fuzz_div_10k() { /* similar pattern */ }
```

**Step 3: Run all fuzz tests**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo test fuzz`
Expected: PASS (or discover bugs to fix)

---

## Task 5: Add edge case coverage (subnormals, special values)

**Files:**
- Modify: `/home/mmacedoeu/_w/ai/cipherocto/determin/src/fuzz.rs`

**Step 1: Add edge case test**

```rust
#[test]
fn test_fuzz_edge_cases() {
    let edge_cases: &[f64] = &[
        0.0, -0.0,
        f64::MIN, f64::MAX, f64::EPSILON,
        f64::MIN_POSITIVE, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        1e-310, 1e-200, 1e-100, 1e10, 1e100,
    ];

    let mut mismatches = Vec::new();

    for &a in edge_cases {
        for &b in edge_cases {
            let dfp_a = Dfp::from_f64(a);
            let dfp_b = Dfp::from_f64(b);

            let (_, soft_add, matches_add) = compare_add(dfp_a, dfp_b);
            if !matches_add {
                mismatches.push(("add", a, b, soft_add));
            }

            // ... same for sub, mul, div
        }
    }

    assert!(mismatches.is_empty(), "Edge case mismatches: {:?}", mismatches);
}
```

**Step 2: Run edge case test**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo test fuzz_edge_cases`
Expected: PASS

---

## Task 6: Run full fuzz suite and commit

**Step 1: Run all tests**

Run: `cd /home/mmacedoeu/_w/ai/cipherocto/determin && cargo test`
Expected: ALL PASS

**Step 2: Commit**

Run:
```bash
cd /home/mmacedoeu/_w/ai/cipherocto
git add determin/src/fuzz.rs determin/Cargo.toml
git commit -m "feat: add differential fuzzing against SoftFloat

- Add softfloat-rs and rand dev-dependencies
- Create fuzz module with compare_* functions
- Add 10k iteration tests for add, sub, mul, div
- Add edge case coverage for subnormals and specials
- CI-ready: fails on any discrepancy"
```
Expected: Commit created

---

## Alternative: If softfloat-rs unavailable

If softfloat-rs crate doesn't exist or doesn't compile, use this fallback:

```rust
// Use standard f64 as reference (not ideal but works for basic fuzzing)
// The real reference should be Berkeley SoftFloat C library compiled to Rust
#[allow(dead_code)]
fn softfloat_reference(a: f64, b: f64, op: &str) -> f64 {
    match op {
        "add" => a + b,
        "sub" => a - b,
        "mul" => a * b,
        "div" => a / b,
        _ => f64::NAN,
    }
}
```

Then update plan to note this is a placeholder until proper SoftFloat binding is found.
