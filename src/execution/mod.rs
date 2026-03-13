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

//! SQL execution engine with gas metering
//!
//! This module provides the execution engine for SQL operations, including
//! gas metering for transaction execution.

pub mod context;
pub mod gas;

pub use context::{ExecutionContext, StateSnapshot};
pub use gas::{GasMeter, GasPrice, GasPrices};

#[cfg(test)]
mod tests;
