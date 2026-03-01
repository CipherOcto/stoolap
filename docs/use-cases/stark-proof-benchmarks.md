# Use Case: STARK Proof Benchmarks with Real STWO

## Problem

The current STARK proof benchmarks use mock proofs that don't reflect real performance. The `STWOProver::generate_mock_proof()` creates fake proof data that validates structure but doesn't exercise actual STWO computation.

This prevents:
- Accurate performance measurement of proof generation/verification
- Comparison between mock and real prover overhead
- Understanding real-world proving times for production planning

## Motivation

### Performance Planning
- Need realistic benchmarks to plan infrastructure capacity
- Can't optimize without measuring real prover overhead
- Mock benchmarks show ~0ms (instant), hiding actual costs

### Integration Validation
- Real STWO integration needs testing beyond unit tests
- Benchmarks provide realistic workload validation
- Continuous performance monitoring

### Feature Parity
- Current implementation is "half" complete - works but unbenchmarked
- Full feature = working implementation + benchmarks
- Required for production readiness

## Impact

After implementation:

1. **Real Benchmarks Available**
   - Measure actual STWO proving time
   - Compare mock vs real overhead
   - Track performance over time

2. **Informed Decisions**
   - Infrastructure capacity planning
   - Batch size optimization
   - Cost estimation

3. **Quality Assurance**
   - Catch performance regressions
   - Validate real integration works
   - Continuous monitoring

## Related RFCs

- [RFC-0201: STWO and Cairo Integration](./rfcs/0201-stwo-cairo-integration.md)
- [RFC-0202: Compressed Proofs](./rfcs/0202-compressed-proofs.md)

## Non-Goals

- This does NOT change the prover API
- This does NOT modify trie or proof structures
- This is infrastructure only - no core logic changes
