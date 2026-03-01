# Use Case: Verifiable State Proofs

## Problem

In blockchain systems, full nodes store the entire state which can be hundreds of gigabytes. Light clients need a way to verify data without downloading everything.

Current Merkle proof systems have limitations:

1. **Binary Tree Assumption** - Most Merkle proofs assume binary trees, but efficient tries use 16-way (hexary) branching
2. **Large Proof Sizes** - Binary proofs for wide tries are inefficient
3. **No Standard Format** - Each project implements custom proof verification

## Motivation

Compact, verifiable state proofs enable:

### Lightweight Clients
- Mobile wallets that don't store full state
- Browser-based blockchain explorers
- IoT devices with limited storage

### Efficient Data Verification
- Proofs that are 5-10x smaller than binary proofs
- Fast verification without full state download
- Bandwidth-efficient protocols

### Interoperability
- Standard proof format across projects
- Cross-chain proof verification
- Composable proof systems

## Impact

If implemented, this enables:

1. **Mobile-First Blockchain Applications** - Apps that run on phones without full nodes
2. **Scalable Verification** - Thousands of proofs verified per second
3. **Cross-Chain Bridges** - State verification between different blockchains
4. **Layer 2 Solutions** - Efficient rollup proof verification

## Technical Context

Hexary tries (16-way branching) are common in blockchain systems:
- **Ethereum** - State trie uses hexary Patricia trie
- **Stoolap** - RowTrie for blockchain SQL

But existing proof formats assume binary trees, creating inefficiency.

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](../../rfcs/0101-hexary-merkle-proofs.md) - The solution
- [Blockchain SQL Database](./blockchain-sql-database.md) - Parent use case

## Success Criteria

- Proofs are <100 bytes for typical operations
- Verification takes <5 microseconds
- Format is standardized and reusable
- Compatible with existing hexary trie structures
