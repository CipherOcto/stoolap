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

//! L2 Rollup Protocol Implementation
//!
//! This module provides data structures and utilities for L2 rollup operations,
//! including batch submission, state management, and withdrawal handling.

pub mod execution;
pub mod fraud;
pub mod submission;
pub mod types;
pub mod withdrawal;

pub use types::{
    FraudProof, RollupBatch, RollupOperation, RollupState, Withdrawal, ADDRESS_SIZE,
    BATCH_INTERVAL, CHALLENGE_PERIOD, MAX_BATCH_SIZE, SEQUENCER_BOND,
};

pub use execution::{ExecutionResult, RollupError, RollupResult};
pub use fraud::{ChallengeContext, ChallengeResult, FraudError, FraudResult};
pub use submission::{SubmissionContext, SubmissionError, SubmissionResult_};
pub use withdrawal::{WithdrawalContext, WithdrawalError, WithdrawalOpResult};
