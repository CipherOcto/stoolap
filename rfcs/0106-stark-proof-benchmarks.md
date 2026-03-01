# RFC-0106: STARK Proof Benchmarks with Real STWO

## Status
Draft

## Summary

Add benchmark infrastructure for STARK proof generation and verification using real STWO prover, with comparison to current mock implementation.

## Motivation

The current `STWOProver::prove()` method uses `generate_mock_proof()` which creates fake proof data. This doesn't reflect real STWO performance, making it impossible to:
- Measure actual proving time
- Compare mock vs real overhead
- Plan production infrastructure

## Specification

### Architecture

```
benches/stark_proof.rs
├── bench_mock_proof_generation    # Using generate_mock_proof()
├── bench_real_proof_generation    # Using stwo-cairo-prover
├── bench_mock_proof_verification  # Verify mock proofs
└── bench_real_proof_verification  # Verify real STWO proofs
```

### Dependencies

Add to `Cargo.toml`:
```toml
stwo-cairo-prover = "1.1"  # Real prover (feature-gated)
```

### Feature Flags

```toml
[features]
default = []
real-stwo = ["dep:stwo-cairo-prover"]
```

### Benchmark Parameters

- **Batch sizes:** 10, 100, 1000 rows
- **Cairo programs:**
  - `hexary_verify.cairo` - Single proof verification
  - `merkle_batch.cairo` - Batch verification
  - `state_transition.cairo` - State transitions

### Cairo Program Compilation

Create `cairo/build.rs` to compile `.cairo` → `.casm` at build time:
```rust
fn main() {
    // Compile Cairo programs to CASM
    compile_cairo("hexary_verify");
    compile_cairo("merkle_batch");
    compile_cairo("state_transition");
}
```

### Prover Integration

Add real proving method to `STWOProver`:

```rust
#[cfg(feature = "real-stwo")]
fn generate_real_proof(
    &self,
    program: &CairoProgram,
    inputs: &[u8],
) -> Result<StarkProof, ProverError> {
    use stwo_cairo_prover::CairoProver;

    let prover = CairoProver::new(self.config.clone());
    prover.prove(program.casm(), inputs)
}
```

### Benchmark Implementation

```rust
fn bench_real_proof_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("stark_proof_generation");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let prover = STWOProver::new();
            let program = load_cairo_program("merkle_batch");
            let inputs = generate_batch_inputs(size);

            b.iter(|| {
                prover.prove(&program, &inputs);
            });
        });
    }
}
```

## Rationale

### Why This Approach?

1. **Feature-gated** - Real STWO only required when benchmarking, not for basic functionality
2. **Comparison ready** - Both mock and real run same benchmark suite
3. **Extensible** - Easy to add new Cairo programs

### Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| A: Replace mock entirely | Simpler code | No comparison possible |
| B: New prover struct | Clean separation | More code duplication |
| C: Feature-gated (chosen) | Both comparisons | Slightly more complex |

## Implementation

### Files to Create

1. `benches/stark_proof.rs` - Benchmark suite
2. `cairo/build.rs` - Cairo compilation

### Files to Modify

1. `Cargo.toml` - Add stwo-cairo-prover dependency
2. `src/zk/prover.rs` - Add `generate_real_proof()` method

### Cairo Programs

| Program | Hash | Purpose |
|--------|------|---------|
| `hexary_verify.cairo` | HEXARY_VERIFY_HASH | Verify hexary proofs |
| `merkle_batch.cairo` | MERKLE_BATCH_HASH | Batch verification |
| `state_transition.cairo` | STATE_TRANSITION_HASH | State transitions |

## Testing Requirements

- [ ] Benchmarks compile with default features (mock only)
- [ ] Benchmarks compile with `--features real-stwo`
- [ ] Mock benchmarks show ~0ms (instant)
- [ ] Real benchmarks show actual proving time
- [ ] Verification benchmarks work for both

## Performance Expectations

- Mock generation: ~0ms (instant)
- Real generation: varies by program size (expected 100ms-10s)
- Mock verification: ~0ms (instant)
- Real verification: expected 10ms-1s

## Related Use Cases

- [STARK Proof Benchmarks](../docs/use-cases/stark-proof-benchmarks.md)

## Related RFCs

- [RFC-0201: STWO and Cairo Integration](./0201-stwo-cairo-integration.md)
- [RFC-0202: Compressed Proofs](./0202-compressed-proofs.md)
