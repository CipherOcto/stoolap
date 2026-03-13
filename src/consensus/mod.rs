// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Consensus module for blockchain operation log
//!
//! This module provides types for representing database operations in the
//! blockchain's operation log, enabling consensus and replication.

pub mod block;
pub mod operation;

pub use block::{Block, BlockError, BlockHeader, BlockOperations};
pub use operation::{ColumnDef, DataType, IndexType, Operation};

#[cfg(test)]
#[path = "tests/operation_tests.rs"]
mod operation_tests;

#[cfg(test)]
#[path = "tests/block_tests.rs"]
mod block_tests;
