# RFC-0103: Blockchain Consensus for SQL Database

## Status
Accepted

## Summary

Define block-based consensus for blockchain SQL database with operations, gas metering, and state root commitments. Enables distributed nodes to agree on database state transitions.

## Motivation

For a blockchain SQL database to work across multiple nodes, they need:

1. **Agreed Ordering** - Which transactions execute first
2. **State Commitment** - Cryptographic commitment to state
3. **Resource Metering** - Prevent infinite loops/exhaustion
4. **Operation Tracking** - Auditable trail of state changes

## Specification

### Block Structure

```rust
pub struct Block {
    pub header: BlockHeader,
    pub operations: BlockOperations,
    pub state_commitment: StateSnapshot,
    pub signatures: Vec<[u8; 32]>,
}

pub struct BlockHeader {
    pub block_number: u64,
    pub parent_hash: [u8; 32],
    pub state_root_before: [u8; 32],
    pub state_root_after: [u8; 32],
    pub operation_root: [u8; 32],
    pub timestamp: i64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub proposer: [u8; 32],
    pub extra_data: Vec<u8>,
}
```

### Operations

```rust
pub enum Operation {
    Insert {
        table_name: String,
        row_id: i64,
        row_data: Vec<u8>,  // Encoded DetermRow
    },
    Update {
        table_name: String,
        row_id: i64,
        column_index: u32,
        old_value: Option<Vec<u8>>,
        new_value: Vec<u8>,
    },
    Delete {
        table_name: String,
        row_id: i64,
    },
    CreateTable {
        schema: TableSchema,
    },
    DropTable {
        table_name: String,
    },
}
```

### Execution Context

```rust
pub struct ExecutionContext {
    block_number: u64,
    gas_limit: u64,
    gas_used: u64,
    state: StateSnapshot,
}

impl ExecutionContext {
    // Execute INSERT
    pub fn insert(&mut self, table: &str, row_id: i64, row: DetermRow) -> Result<([u8; 32], [u8; 32])>;

    // Execute UPDATE
    pub fn update(&mut self, table: &str, row_id: i64, column: u32, value: DetermValue) -> Result<([u8; 32], [u8; 32])>;

    // Execute DELETE
    pub fn delete(&mut self, table: &str, row_id: i64) -> Result<()>;

    // Get gas used
    pub fn gas_used(&self) -> u64;

    // Get final state
    pub fn into_state(self) -> StateSnapshot;

    // Get schema root
    pub fn schema_root(&self) -> [u8; 32];
}
```

### Gas Metering

| Operation | Gas Cost |
|-----------|----------|
| INSERT | 1000 |
| UPDATE | 500 |
| DELETE | 500 |
| CREATE TABLE | 5000 |
| DROP TABLE | 1000 |

### State Commitment

```rust
pub struct StateSnapshot {
    pub schemas: SchemaTrie,
    pub tables: BTreeMap<String, RowTrie>,
}
```

State roots computed from:
- `SchemaTrie` root for table metadata
- Each `RowTrie` root for table data
- Combined into overall schema root

### Verification

```rust
impl Block {
    pub fn verify(&self) -> Result<()> {
        // 1. Verify operation root matches operations
        let expected_op_root = BlockOperations::compute_operation_root(&self.operations.operations);
        if expected_op_root != self.header.operation_root {
            return Err(Error::InvalidOperationRoot);
        }

        // 2. Verify state root after matches execution
        let mut ctx = ExecutionContext::new(
            self.header.block_number,
            self.header.gas_limit,
            self.state_commitment.clone(),
        );

        // Execute operations
        for op in &self.operations.operations {
            match op {
                Operation::Insert { table_name, row_id, row_data } => {
                    let row = DetermRow::decode(row_data)?;
                    ctx.insert(table_name, *row_id, row)?;
                }
                // ... handle other operation types
            }
        }

        // Verify final state root
        if ctx.schema_root() != self.header.state_root_after {
            return Err(Error::InvalidStateRoot);
        }

        // 3. Verify gas used
        if ctx.gas_used() != self.header.gas_used {
            return Err(Error::InvalidGasUsed);
        }

        Ok(())
    }
}
```

## Rationale

### Why Block-Based?

1. **Batching** - Multiple operations per block
2. **Ordering** - Total order within block
3. **Atomicity** - All operations succeed or none
4. **Efficiency** - Amortize verification cost

### Why Gas Metering?

1. **Denial of Service Prevention** - Limit computation
2. **Fair Resource Allocation** - Pay for what you use
3. **Economic Security** - Attach cost to state changes

### Why Operation Encoding?

1. **Deterministic Replay** - Any node can replay operations
2. **Auditing** - Clear trail of state changes
3. **Compression** - Binary format more compact than SQL

## Implementation

### Components

1. **Block Types** - Block, BlockHeader, BlockOperations, Operation
2. **ExecutionContext** - Gas-metered execution with state
3. **Operation Execution** - INSERT, UPDATE, DELETE logic
4. **Verification** - Block verification against state commitment
5. **Encoding/Decoding** - Binary format for operations

### Gas Cost Formula

```
total_gas = Σ(operation_costs) + overhead

where:
- INSERT: 1000 gas
- UPDATE: 500 gas
- DELETE: 500 gas
- overhead: block header processing cost
```

### State Root Calculation

```
state_root = SHA256(
    schema_trie_root ||
    row_trie_root_1 ||
    row_trie_root_2 ||
    ...
)
```

Where:
- `schema_trie_root` = Root of SchemaTrie (table metadata)
- `row_trie_root_N` = Root of RowTrie for table N

## Security Considerations

1. **Block Verification** - All blocks verified before acceptance
2. **Gas Limits** - Block gas limit prevents infinite loops
3. **State Validation** - State roots must match execution
4. **Operation Replay** - Operations must be valid and deterministic
5. **Timestamp Ordering** - Blocks must have increasing timestamps

## Consensus Algorithm

This RFC defines the block structure but defers consensus algorithm choice to:
- Proof of Authority (initial)
- Future: Proof of Stake, Tendermint, etc.

## Backward Compatibility

Blocks are versioned. Future versions can:
- Add new operation types
- Modify gas costs
- Change state root calculation

## Related Use Cases

- [Blockchain SQL Database](../../docs/use-cases/blockchain-sql-database.md)

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](./0101-hexary-merkle-proofs.md)
- [RFC-0102: Deterministic Value Types](./0102-deterministic-types.md)
