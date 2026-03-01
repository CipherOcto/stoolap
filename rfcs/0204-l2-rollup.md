# RFC-0204: L2 Rollup Protocol

## Status
Draft

## Summary

Define a rollup protocol that executes thousands of transactions off-chain and posts only validity proofs to Stoolap Chain. Enables 1000+ TPS while inheriting Stoolap's security.

## Motivation

Current Stoolap Chain executes every transaction on-chain, limiting throughput:
- **TPS Bottleneck**: Each transaction consumes gas and block space
- **Cost Scaling**: Fees increase with network usage
- **No Parallelism**: Sequential execution limits throughput

Rollup approach:
- **Off-Chain Execution**: Execute transactions in rollup, not on chain
- **On-Chain Verification**: Post only validity proof
- **Batch Proofs**: Prove thousands of transactions in one STARK proof
- **Economic Security**: Stakers can challenge invalid batches

## Specification

### Data Structures

```rust
/// Rollup batch containing multiple transactions
pub struct RollupBatch {
    pub batch_number: u64,
    pub parent_hash: [u8; 32],      // Hash of previous batch
    pub transactions: Vec<Transaction>, // Off-chain transactions
    pub pre_state_root: [u8; 32],   // State before batch
    pub post_state_root: [u8; 32],  // State after batch
    pub timestamp: u64,             // Batch creation time
}

/// Rollup state commitment
pub struct RollupState {
    pub batch_number: u64,
    pub state_root: [u8; 32],       // Latest state root
    pub pending_withdrawals: Vec<Withdrawal>,
    pub sequencer: Address,         // Current sequencer
}

/// Withdrawal from L2 to L1
pub struct Withdrawal {
    pub recipient: Address,
    pub amount: u64,
    pub batch_number: u64,          // When withdrawal initiated
}

/// Batch submission operation
pub enum RollupOperation {
    SubmitBatch {
        batch: RollupBatch,
        proof: StarkProof,         // Proof of valid execution
    },
    ChallengeBatch {
        batch_number: u64,
        proof: FraudProof,         // Proof batch is invalid
    },
    FinalizeWithdrawal {
        withdrawal_id: u64,
    },
}
```

### Cairo Program: `rollup_verify.cairo`

```cairo
// rollup_verify.cairo - Verify rollup batch execution

#[derive(Drop, Serde)]
struct Transaction {
    sender: u256,
    operation: u8,      // 0=INSERT, 1=UPDATE, 2=DELETE
    table: u256,        // Table identifier
    row_id: u64,
    value: u256,
    nonce: u64,
}

#[derive(Drop, Serde)]
struct RollupBatch {
    batch_number: u64,
    parent_hash: u256,
    transactions: Array<Transaction>,
    pre_state_root: u256,
    post_state_root: u256,
}

// Verify rollup batch was executed correctly
fn verify_rollup_batch(
    batch: RollupBatch,
    expected_pre_root: u256,
) -> bool {
    // Verify batch starts from correct state
    if batch.pre_state_root != expected_pre_root {
        return false;
    }

    // Execute all transactions
    let mut current_root = batch.pre_state_root;
    let mut i = 0;

    while i < batch.transactions.len() {
        let tx = batch.transactions[i];

        // Execute transaction
        let new_root = execute_transaction(current_root, tx);
        current_root = new_root;

        i += 1;
    }

    // Verify final state matches claimed
    current_root == batch.post_state_root
}

// Execute single transaction
fn execute_transaction(
    state_root: u256,
    tx: Transaction,
) -> u256 {
    match tx.operation {
        0 => execute_insert(state_root, tx),
        1 => execute_update(state_root, tx),
        2 => execute_delete(state_root, tx),
        _ => state_root, // Invalid op, state unchanged
    }
}

// Execute INSERT operation
fn execute_insert(state_root: u256, tx: Transaction) -> u256 {
    // Compute new state root with row inserted
    let row_hash = hash_row(tx.table, tx.row_id, tx.value);

    // Update trie with new row
    let new_root = merkle_insert(state_root, tx.table, tx.row_id, row_hash);

    new_root
}

// Execute UPDATE operation
fn execute_update(state_root: u256, tx: Transaction) -> u256 {
    let new_value_hash = hash_row(tx.table, tx.row_id, tx.value);

    // Update trie with new value
    let new_root = merkle_update(state_root, tx.table, tx.row_id, new_value_hash);

    new_root
}

// Execute DELETE operation
fn execute_delete(state_root: u256, tx: Transaction) -> u256 {
    // Remove row from trie
    let new_root = merkle_delete(state_root, tx.table, tx.row_id);

    new_root
}

// Hash row data
fn hash_row(table: u256, row_id: u64, value: u256) -> u256 {
    poseidon(table, row_id, value)
}

// Merkle insert (simplified)
fn merkle_insert(
    root: u256,
    table: u256,
    row_id: u64,
    value_hash: u256,
) -> u256 {
    // In reality, this would use hexary trie operations
    poseidon(root, table, row_id, value_hash)
}

// Merkle update (simplified)
fn merkle_update(
    root: u256,
    table: u256,
    row_id: u64,
    new_value_hash: u256,
) -> u256 {
    poseidon(root, table, row_id, new_value_hash)
}

// Merkle delete (simplified)
fn merkle_delete(root: u256, table: u256, row_id: u64) -> u256 {
    poseidon(root, table, row_id, 0)
}
```

### Rollup Operation

```rust
impl RollupBatch {
    /// Execute batch off-chain and generate proof
    pub fn execute_and_prove(
        &self,
        pre_state_root: [u8; 32],
        program: &CairoProgram,
    ) -> Result<StarkProof, RollupError> {
        // Verify parent chain
        if self.parent_hash != get_latest_batch_hash() {
            return Err(RollupError::InvalidParent);
        }

        // Verify pre-state
        if self.pre_state_root != pre_state_root {
            return Err(RollupError::InvalidPreState);
        }

        // Execute transactions
        let mut state = RollupState::new(pre_state_root);
        for tx in &self.transactions {
            state.execute_transaction(tx)?;
        }

        // Verify post-state
        if state.root != self.post_state_root {
            return Err(RollupError::InvalidPostState);
        }

        // Generate proof
        let prover = STWOProver::new();
        let input = serialize_batch_input(self, pre_state_root);
        let proof = prover.prove(program, &input)?;

        Ok(proof)
    }
}
```

### Batch Submission

```rust
impl ExecutionContext {
    /// Submit rollup batch to chain
    pub fn submit_rollup_batch(
        &mut self,
        batch: RollupBatch,
        proof: StarkProof,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Verify sequencer is authorized
        if !self.is_authorized_sequencer(batch.sequencer) {
            return Err(ExecutionError::UnauthorizedSequencer);
        }

        // Verify batch number
        let expected_number = self.get_next_batch_number();
        if batch.batch_number != expected_number {
            return Err(ExecutionError::InvalidBatchNumber);
        }

        // Verify proof
        let program = self.get_rollup_program()?;
        let verifier = STWOVerifier::new();
        if !verifier.verify(&proof)? {
            return Err(ExecutionError::InvalidProof);
        }

        // Update rollup state
        self.rollup_state = RollupState {
            batch_number: batch.batch_number,
            state_root: batch.post_state_root,
            pending_withdrawals: Vec::new(),
            sequencer: batch.sequencer,
        };

        Ok(ExecutionResult {
            gas_used: 100_000,
            logs: vec!["Batch submitted".to_string()],
        })
    }

    /// Challenge invalid batch (fraud proof)
    pub fn challenge_batch(
        &mut self,
        batch_number: u64,
        fraud_proof: FraudProof,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Verify fraud proof
        if !self.verify_fraud_proof(&fraud_proof) {
            return Err(ExecutionError::InvalidFraudProof);
        }

        // Slash sequencer stake
        self.slash_sequencer(fraud_proof.sequencer);

        // Revert batch and all descendants
        self.revert_batches_from(batch_number);

        Ok(ExecutionResult {
            gas_used: 50_000,
            logs: vec!["Batch reverted".to_string()],
        })
    }
}
```

### Fraud Proof

```rust
/// Proof that batch execution was incorrect
pub struct FraudProof {
    pub batch_number: u64,
    pub transaction_index: u64,
    pub pre_state_root: [u8; 32],
    pub expected_post_root: [u8; 32],  // Correct post-state
    pub claimed_post_root: [u8; 32],   // What sequencer claimed
    pub proof: MerkleProof,            // Proof of state access
}

impl FraudProof {
    /// Verify fraud proof
    pub fn verify(&self) -> bool {
        // Re-execute transaction
        let expected_root = execute_transaction_with_proof(
            self.pre_state_root,
            self.transaction_index,
            &self.proof,
        );

        // Verify claimed root is wrong
        self.claimed_post_root != expected_root
    }
}
```

### Withdrawal Flow

```rust
impl ExecutionContext {
    /// Initiate withdrawal from L2 to L1
    pub fn initiate_withdrawal(
        &mut self,
        recipient: Address,
        amount: u64,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Create withdrawal record
        let withdrawal = Withdrawal {
            recipient,
            amount,
            batch_number: self.rollup_state.batch_number + CHALLENGE_PERIOD,
        };

        self.rollup_state.pending_withdrawals.push(withdrawal);

        Ok(ExecutionResult {
            gas_used: 20_000,
            logs: vec!["Withdrawal initiated".to_string()],
        })
    }

    /// Finalize withdrawal after challenge period
    pub fn finalize_withdrawal(
        &mut self,
        withdrawal_id: u64,
    ) -> Result<ExecutionResult, ExecutionError> {
        let withdrawal = self.rollup_state.pending_withdrawals
            .get(withdrawal_id as usize)
            .ok_or(ExecutionError::InvalidWithdrawal)?;

        // Verify challenge period passed
        if withdrawal.batch_number > self.rollup_state.batch_number {
            return Err(ExecutionError::ChallengePeriodNotPassed);
        }

        // Transfer funds to recipient
        self.transfer_balance(withdrawal.recipient, withdrawal.amount)?;

        // Remove withdrawal
        self.rollup_state.pending_withdrawals.remove(withdrawal_id as usize);

        Ok(ExecutionResult {
            gas_used: 30_000,
            logs: vec!["Withdrawal finalized".to_string()],
        })
    }
}
```

### Constants

```rust
const CHALLENGE_PERIOD: u64 = 100;        // Batches
const MAX_BATCH_SIZE: usize = 10000;      // Transactions per batch
const BATCH_INTERVAL: u64 = 10;           // Seconds between batches
const SEQUENCER_BOND: u64 = 100_000;      // Minimum stake
const SLASH_FRACTION: u64 = 10;           // 1/10 of bond slashed
```

### Sequencer Selection

```rust
impl ExecutionContext {
    /// Get current sequencer (round-robin or stake-weighted)
    pub fn get_current_sequencer(&self) -> Address {
        // Option 1: Round-robin through authorized sequencers
        let sequencers = self.get_authorized_sequencers();
        let index = (self.rollup_state.batch_number as usize) % sequencers.len();
        sequencers[index]

        // Option 2: Stake-weighted random selection
        // let total_stake = self.get_total_sequencer_stake();
        // let random = self.get_random_value() % total_stake;
        // self.select_sequencer_by_stake(random)
    }
}
```

## Rationale

### Why Optimistic Rollup?

1. **Fast Proofs** - Don't need to generate STARK proof for every batch
2. **Economic Security** - Frauds are economically disincentivized
3. **Flexibility** - Can support any computation (not limited to STARK-friendly)

### Why Challenge Period?

1. **Finality** - Allows time for fraud proofs
2. **Security** - Prevents invalid state finalization
3. **Liquidity** - Withdrawals can't be stolen

### Why Sequencer Bond?

1. **Skin in the Game** - Sequencers have economic stake
2. **Slashing** - Can punish malicious behavior
3. **Sybil Resistance** - Expensive to be sequencer

## Implementation

### Components

1. **Cairo Program** - `rollup_verify.cairo`
2. **Rollup State** - Batch tracking and state roots
3. **Batch Execution** - Off-chain transaction execution
4. **Proof Generation** - STARK proof for validity
5. **Fraud Proofs** - Challenge mechanism
6. **Withdrawals** - L2 to L1 transfers

### Dependencies

- Requires: RFC-0201, RFC-0202, RFC-0203
- Enables: High-throughput applications

### Testing Requirements

- Unit tests for all components
- Integration tests for full flow
- Fraud proof tests
- Challenge period tests
- Benchmarks for throughput

## Performance Targets

| Metric | Target | Actual (TBD) |
|--------|--------|--------------|
| TPS | >1000 | TBD |
| Batch interval | 10s | TBD |
| Proof generation | <30s | TBD |
| Finality time | <1 min | TBD |
| Batch proof size | <50 KB | TBD |

## Security Considerations

1. **Fraud Proof Validity** - Must be verifiable on-chain
2. **Sequencer Bond** - Sufficient to disincentivize fraud
3. **Challenge Period** - Long enough to detect fraud
4. **State Availability** - Data must be accessible for fraud proofs
5. **Sequencer Selection** - Prevent centralization

## Economic Considerations

1. **Sequencer Revenue** - Transaction fees + MEV
2. **Bond Requirements** - Balance security vs accessibility
3. **Fee Distribution** - How to share revenue
4. **Slashing** - What portion of bond to slash

## Backward Compatibility

- L1 transactions unchanged
- Rollup is opt-in layer
- No breaking changes

## Related Use Cases

- [ZK Proofs for Scalability and Privacy](../../docs/use-cases/zk-proofs-scalability.md)

## Related RFCs

- [RFC-0201: STWO/Cairo Integration](./0201-stwo-cairo-integration.md)
- [RFC-0202: Compressed Proofs](./0202-compressed-proofs.md)
- [RFC-0203: Confidential Queries](./0203-confidential-queries.md)

## Open Questions

1. What sequencer selection algorithm to use?
2. How long should the challenge period be?
3. Should we support ZK rollup instead of optimistic?
4. How do we handle data availability?

## References

- [Optimistic Rollup](https://www.optimism.io/)
- [Arbitrum Rollup](https://developer.offchainlabs.com/)
- [ZK Rollup](https://zksync.io/)
