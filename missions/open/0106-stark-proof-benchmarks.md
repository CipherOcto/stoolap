# Mission: STARK Proof Benchmarks with Real STWO

## Status
Open

## RFC
RFC-0106

## Acceptance Criteria

- [ ] Add `stwo-cairo-prover` dependency to Cargo.toml
- [ ] Create `cairo/build.rs` for Cairo compilation
- [ ] Add `generate_real_proof()` method to STWOProver
- [ ] Create `benches/stark_proof.rs` with 8 benchmarks (all require zk):
  - [ ] bench_mock_proof_generation (requires zk)
  - [ ] bench_real_proof_generation (requires zk)
  - [ ] bench_mock_proof_verification (requires zk)
  - [ ] bench_real_proof_verification (requires zk)
- [ ] Benchmark all 3 Cairo programs (hexary_verify, merkle_batch, state_transition)
- [ ] Run batch sizes: 10, 100, 1000
- [ ] Verify benchmarks compile with `--features zk`

## Dependencies

- RFC-0201 (STWO Integration) - Complete
- Mission 0201-01 (STWO Dependency) - Complete

## Enables

- Performance monitoring for production
- Infrastructure capacity planning

## Implementation Details

### Files to Create

1. **benches/stark_proof.rs** - New benchmark suite
   ```rust
   // 4 benchmarks, 3 programs, 3 batch sizes each = 36 benchmark cases
   ```

2. **cairo/build.rs** - Compile Cairo to CASM
   ```rust
   // Compile hexary_verify.cairo, merkle_batch.cairo, state_transition.cairo
   ```

### Files to Modify

1. **Cargo.toml**
   ```toml
   stwo-cairo-prover = { version = "1.1", optional = true }

   [features]
   default = []
   real-stwo = ["dep:stwo-cairo-prover"]
   ```

2. **src/zk/prover.rs**
   ```rust
   #[cfg(feature = "real-stwo")]
   fn generate_real_proof(&self, program: &CairoProgram, inputs: &[u8]) -> Result<StarkProof, ProverError>
   ```

### Cairo Programs

| Program | Purpose |
|---------|---------|
| hexary_verify.cairo | Single hexary proof verification |
| merkle_batch.cairo | Batch proof verification |
| state_transition.cairo | State transition verification |

### Benchmark Structure

```
stark_proof_generation (mock)
├── 10 rows
├── 100 rows
└── 1000 rows

stark_proof_generation (real)
├── 10 rows
├── 100 rows
└── 1000 rows

stark_proof_verification (mock)
├── 10 rows
├── 100 rows
└── 1000 rows

stark_proof_verification (real)
├── 10 rows
├── 100 rows
└── 1000 rows
```

## Test Commands

```bash
# Run all benchmarks (requires zk)
cargo bench --bench stark_proof --features zk
```

## Claimant

None

## Pull Request

#

## Notes

### Why This Matters

- Current mock benchmarks show ~0ms - not realistic
- Real benchmarks needed for production planning
- Comparison shows actual STWO overhead

### Expected Results

- Mock: ~0ms (instant)
- Real: varies by program (100ms - 10s)
- Verification typically faster than generation
