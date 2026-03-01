# Mission: L2 Rollup Data Structures

## Status
Open

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [ ] Define `RollupBatch` struct
- [ ] Define `RollupState` struct
- [ ] Define `Withdrawal` struct
- [ ] Define `RollupOperation` enum
- [ ] Define `FraudProof` struct
- [ ] Implement SolanaSerialize for all types
- [ ] Add tests for batch encoding/decoding

## Dependencies
- RFC-0201 (STWO Integration) - Complete
- RFC-0103 (Blockchain Consensus) - Complete

## Enables
- Mission 0204-02 (Batch Execution)

## Implementation Notes

**Files to Create:**
- `src/rollup/mod.rs` - Rollup module
- `src/rollup/types.rs` - Rollup types

**Data Structures:**
```rust
pub struct RollupBatch {
    pub batch_number: u64,
    pub parent_hash: [u8; 32],
    pub transactions: Vec<Transaction>,
    pub pre_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
    pub timestamp: u64,
}

pub struct RollupState {
    pub batch_number: u64,
    pub state_root: [u8; 32],
    pub pending_withdrawals: Vec<Withdrawal>,
    pub sequencer: Address,
}

pub struct Withdrawal {
    pub recipient: Address,
    pub amount: u64,
    pub batch_number: u64,
}

pub enum RollupOperation {
    SubmitBatch {
        batch: RollupBatch,
        proof: StarkProof,
    },
    ChallengeBatch {
        batch_number: u64,
        proof: FraudProof,
    },
    FinalizeWithdrawal {
        withdrawal_id: u64,
    },
}

pub struct FraudProof {
    pub batch_number: u64,
    pub transaction_index: u64,
    pub pre_state_root: [u8; 32],
    pub expected_post_root: [u8; 32],
    pub claimed_post_root: [u8; 32],
    pub proof: MerkleProof,
}
```

**Constants:**
```rust
const CHALLENGE_PERIOD: u64 = 100;
const MAX_BATCH_SIZE: usize = 10000;
const BATCH_INTERVAL: u64 = 10;
const SEQUENCER_BOND: u64 = 100_000;
```

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
