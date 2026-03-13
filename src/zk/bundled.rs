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

//! Bundled Cairo Programs
//!
//! Pre-compiled Cairo programs that ship with Stoolap.
//! These are compiled to CASM and included as binary data.

use crate::zk::cairo::{CairoProgram, CairoProgramHash, CairoProgramRegistry};

// Program hashes (computed from source)
// These are placeholders - real hashes should be computed from compiled CASM

/// Hash for state_transition.cairo program
pub const STATE_TRANSITION_HASH: CairoProgramHash = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
];

/// Hash for hexary_verify.cairo program
pub const HEXARY_VERIFY_HASH: CairoProgramHash = [
    0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40,
];

/// Hash for merkle_batch.cairo program
pub const MERKLE_BATCH_HASH: CairoProgramHash = [
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
    0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60,
];

// Bundled CASM bytecode
// These are minimal stubs - real programs should have actual compiled CASM

/// Compiled CASM for state_transition.cairo
pub const STATE_TRANSITION_CASM: &[u8] = &[
    // Minimal CASM stub - should be replaced with actual compiled bytecode
    0x01, 0x02, 0x03, 0x04,
];

/// Compiled CASM for hexary_verify.cairo
pub const HEXARY_VERIFY_CASM: &[u8] = &[
    // Minimal CASM stub - should be replaced with actual compiled bytecode
    0x11, 0x12, 0x13, 0x14,
];

/// Compiled CASM for merkle_batch.cairo
pub const MERKLE_BATCH_CASM: &[u8] = &[
    // Minimal CASM stub - should be replaced with actual compiled bytecode
    0x21, 0x22, 0x23, 0x24,
];

/// Register all bundled Cairo programs with the registry
pub fn register_bundled_programs(registry: &mut CairoProgramRegistry) -> Result<(), BundledError> {
    // Register state_transition.cairo
    let state_transition = CairoProgram {
        hash: STATE_TRANSITION_HASH,
        source: include_str!("../../cairo/state_transition.cairo").to_string(),
        sierra: vec![],
        casm: STATE_TRANSITION_CASM.to_vec(),
        version: 2, // Cairo 2.0
    };
    registry.register(state_transition).map_err(|e| {
        BundledError::RegistrationFailed("state_transition".to_string(), e.to_string())
    })?;

    // Register hexary_verify.cairo
    let hexary_verify = CairoProgram {
        hash: HEXARY_VERIFY_HASH,
        source: include_str!("../../cairo/hexary_verify.cairo").to_string(),
        sierra: vec![],
        casm: HEXARY_VERIFY_CASM.to_vec(),
        version: 2,
    };
    registry.register(hexary_verify).map_err(|e| {
        BundledError::RegistrationFailed("hexary_verify".to_string(), e.to_string())
    })?;

    // Register merkle_batch.cairo
    let merkle_batch = CairoProgram {
        hash: MERKLE_BATCH_HASH,
        source: include_str!("../../cairo/merkle_batch.cairo").to_string(),
        sierra: vec![],
        casm: MERKLE_BATCH_CASM.to_vec(),
        version: 2,
    };
    registry
        .register(merkle_batch)
        .map_err(|e| BundledError::RegistrationFailed("merkle_batch".to_string(), e.to_string()))?;

    Ok(())
}

/// Get a bundled program by name
pub fn get_bundled_program(name: &str) -> Option<CairoProgram> {
    match name {
        "state_transition" => Some(CairoProgram {
            hash: STATE_TRANSITION_HASH,
            source: include_str!("../../cairo/state_transition.cairo").to_string(),
            sierra: vec![],
            casm: STATE_TRANSITION_CASM.to_vec(),
            version: 2,
        }),
        "hexary_verify" => Some(CairoProgram {
            hash: HEXARY_VERIFY_HASH,
            source: include_str!("../../cairo/hexary_verify.cairo").to_string(),
            sierra: vec![],
            casm: HEXARY_VERIFY_CASM.to_vec(),
            version: 2,
        }),
        "merkle_batch" => Some(CairoProgram {
            hash: MERKLE_BATCH_HASH,
            source: include_str!("../../cairo/merkle_batch.cairo").to_string(),
            sierra: vec![],
            casm: MERKLE_BATCH_CASM.to_vec(),
            version: 2,
        }),
        _ => None,
    }
}

/// Check if a program hash is from a bundled program
pub fn is_bundled_program(hash: &CairoProgramHash) -> bool {
    *hash == STATE_TRANSITION_HASH || *hash == HEXARY_VERIFY_HASH || *hash == MERKLE_BATCH_HASH
}

/// Get the name of a bundled program from its hash
pub fn get_bundled_program_name(hash: &CairoProgramHash) -> Option<&'static str> {
    if *hash == STATE_TRANSITION_HASH {
        Some("state_transition")
    } else if *hash == HEXARY_VERIFY_HASH {
        Some("hexary_verify")
    } else if *hash == MERKLE_BATCH_HASH {
        Some("merkle_batch")
    } else {
        None
    }
}

/// Errors during bundled program operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundledError {
    /// Failed to register a bundled program
    RegistrationFailed(String, String),
    /// Bundled program not found
    ProgramNotFound(String),
}

impl std::fmt::Display for BundledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundledError::RegistrationFailed(name, msg) => {
                write!(f, "Failed to register bundled program '{}': {}", name, msg)
            }
            BundledError::ProgramNotFound(name) => {
                write!(f, "Bundled program '{}' not found", name)
            }
        }
    }
}

impl std::error::Error for BundledError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_bundled_programs() {
        let mut registry = CairoProgramRegistry::new();
        assert!(register_bundled_programs(&mut registry).is_ok());

        // Verify all programs are registered
        assert!(registry.get(&STATE_TRANSITION_HASH).is_some());
        assert!(registry.get(&HEXARY_VERIFY_HASH).is_some());
        assert!(registry.get(&MERKLE_BATCH_HASH).is_some());
    }

    #[test]
    fn test_get_bundled_program() {
        let program = get_bundled_program("state_transition");
        assert!(program.is_some());
        assert_eq!(program.unwrap().hash, STATE_TRANSITION_HASH);

        let program = get_bundled_program("nonexistent");
        assert!(program.is_none());
    }

    #[test]
    fn test_is_bundled_program() {
        assert!(is_bundled_program(&STATE_TRANSITION_HASH));
        assert!(is_bundled_program(&HEXARY_VERIFY_HASH));
        assert!(is_bundled_program(&MERKLE_BATCH_HASH));

        let unknown_hash = [0u8; 32];
        assert!(!is_bundled_program(&unknown_hash));
    }

    #[test]
    fn test_get_bundled_program_name() {
        assert_eq!(
            get_bundled_program_name(&STATE_TRANSITION_HASH),
            Some("state_transition")
        );
        assert_eq!(
            get_bundled_program_name(&HEXARY_VERIFY_HASH),
            Some("hexary_verify")
        );
        assert_eq!(
            get_bundled_program_name(&MERKLE_BATCH_HASH),
            Some("merkle_batch")
        );

        let unknown_hash = [0u8; 32];
        assert_eq!(get_bundled_program_name(&unknown_hash), None);
    }

    #[test]
    fn test_bundled_error_display() {
        let err = BundledError::ProgramNotFound("test".to_string());
        let display = format!("{}", err);
        assert!(display.contains("test"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_bundled_programs_have_source() {
        let state_transition = get_bundled_program("state_transition").unwrap();
        assert!(!state_transition.source.is_empty());
        // Check for copyright header
        assert!(state_transition.source.contains("Copyright"));
        assert!(state_transition.source.contains("State Transition"));

        let hexary_verify = get_bundled_program("hexary_verify").unwrap();
        assert!(!hexary_verify.source.is_empty());
        assert!(hexary_verify.source.contains("Copyright"));
        assert!(hexary_verify.source.contains("Hexary"));

        let merkle_batch = get_bundled_program("merkle_batch").unwrap();
        assert!(!merkle_batch.source.is_empty());
        assert!(merkle_batch.source.contains("Copyright"));
        assert!(merkle_batch.source.contains("Batch"));
    }

    #[test]
    fn test_all_bundled_programs_version_2() {
        let state_transition = get_bundled_program("state_transition").unwrap();
        assert_eq!(state_transition.version, 2);

        let hexary_verify = get_bundled_program("hexary_verify").unwrap();
        assert_eq!(hexary_verify.version, 2);

        let merkle_batch = get_bundled_program("merkle_batch").unwrap();
        assert_eq!(merkle_batch.version, 2);
    }

    #[test]
    fn test_bundled_program_hashes_are_unique() {
        assert_ne!(STATE_TRANSITION_HASH, HEXARY_VERIFY_HASH);
        assert_ne!(STATE_TRANSITION_HASH, MERKLE_BATCH_HASH);
        assert_ne!(HEXARY_VERIFY_HASH, MERKLE_BATCH_HASH);
    }
}
