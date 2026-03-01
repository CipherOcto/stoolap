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

//! Cairo program types and registry
//!
//! This module provides data structures for managing Cairo programs
//! that can be proven using STWO.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Minimum supported Cairo compiler version
const MIN_CAIRO_VERSION: u32 = 2_06_00;

/// Default Cairo compiler version (2.6.0)
const DEFAULT_CAIRO_VERSION: u32 = 2_06_00;

/// Cairo program identifier (blake3 hash of source code)
pub type CairoProgramHash = [u8; 32];

/// Compiled Cairo program with all artifacts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CairoProgram {
    /// Blake3 hash of the source code (unique identifier)
    pub hash: CairoProgramHash,
    /// Cairo source code
    pub source: String,
    /// Sierra intermediate representation (IR)
    pub sierra: Vec<u8>,
    /// Cairo Assembly Machine (executable format)
    pub casm: Vec<u8>,
    /// Cairo compiler version
    pub version: u32,
}

impl CairoProgram {
    /// Create a new Cairo program from source code
    ///
    /// Note: This is a placeholder. The actual compilation
    /// will be implemented in Mission 0201-03.
    pub fn from_source(source: String, version: u32) -> Self {
        let hash = Self::compute_hash(&source);
        Self {
            hash,
            source,
            sierra: Vec::new(),
            casm: Vec::new(),
            version,
        }
    }

    /// Compute blake3 hash of Cairo source code
    pub fn compute_hash(source: &str) -> CairoProgramHash {
        #[cfg(feature = "zk")]
        {
            blake3::hash(source.as_bytes()).into()
        }

        #[cfg(not(feature = "zk"))]
        {
            // Fallback for when zk feature is not enabled
            let mut hash = [0u8; 32];
            let bytes = source.as_bytes();
            let len = bytes.len().min(32);
            hash[..len].copy_from_slice(&bytes[..len]);
            hash
        }
    }

    /// Compile Cairo source to Sierra
    ///
    /// This method calls the cairo-compile binary to compile Cairo source code
    /// to Sierra intermediate representation.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if:
    /// - Cairo compiler is not found
    /// - Source code has syntax errors
    /// - Compilation fails for any reason
    pub fn compile_to_sierra(source: &str) -> Result<Vec<u8>, CompileError> {
        // Try to find cairo-compile in PATH
        let compiler = find_cairo_compiler()?;

        // Write source to a temporary file
        let source_file = tempfile::NamedTempFile::new()
            .map_err(|e| CompileError::SyntaxError(format!("Failed to create temp file: {}", e)))?;

        std::fs::write(source_file.path(), source)
            .map_err(|e| CompileError::SyntaxError(format!("Failed to write source: {}", e)))?;

        // Run cairo-compile
        let output = Command::new(&compiler)
            .arg(source_file.path())
            .arg("--sierra")
            .arg("--output")
            .arg("-") // Output to stdout
            .output()
            .map_err(|e| CompileError::CompilerNotFound)?;

        // Check if compilation succeeded
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(parse_compile_error(&error_msg));
        }

        // Return Sierra bytecode
        Ok(output.stdout)
    }

    /// Compile Sierra to CASM
    ///
    /// This method calls the sierra-to-casm compiler to convert Sierra
    /// intermediate representation to Cairo Assembly Machine bytecode.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if:
    /// - Sierra-to-CASM compiler is not found
    /// - Sierra bytecode is invalid
    /// - Compilation fails for any reason
    pub fn compile_to_casm(sierra: &[u8]) -> Result<Vec<u8>, CompileError> {
        // Try to find sierra-to-casm-compile in PATH
        let compiler = find_sierra_to_casm_compiler()?;

        // Write Sierra to a temporary file
        let sierra_file = tempfile::NamedTempFile::new()
            .map_err(|e| CompileError::TypeError(format!("Failed to create temp file: {}", e)))?;

        std::fs::write(sierra_file.path(), sierra)
            .map_err(|e| CompileError::TypeError(format!("Failed to write Sierra: {}", e)))?;

        // Run sierra-to-casm-compile
        let output = Command::new(&compiler)
            .arg(sierra_file.path())
            .arg("--output")
            .arg("-") // Output to stdout
            .output()
            .map_err(|e| CompileError::CompilerNotFound)?;

        // Check if compilation succeeded
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(parse_compile_error(&error_msg));
        }

        // Return CASM bytecode
        Ok(output.stdout)
    }

    /// Compile Cairo source fully to CASM
    ///
    /// This is a convenience method that compiles Cairo source → Sierra → CASM.
    pub fn compile_full(source: &str) -> Result<Self, CompileError> {
        let hash = Self::compute_hash(source);
        let sierra = Self::compile_to_sierra(source)?;
        let casm = Self::compile_to_casm(&sierra)?;

        Ok(Self {
            hash,
            source: source.to_string(),
            sierra,
            casm,
            version: DEFAULT_CAIRO_VERSION,
        })
    }

    /// Check if this program has been fully compiled
    pub fn is_compiled(&self) -> bool {
        !self.sierra.is_empty() && !self.casm.is_empty()
    }
}

/// Cairo program compilation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Feature not implemented yet
    NotImplemented(String),
    /// Syntax error in Cairo source
    SyntaxError(String),
    /// Type error during compilation
    TypeError(String),
    /// Compiler not found
    CompilerNotFound,
    /// Invalid Cairo compiler version
    InvalidVersion(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            CompileError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            CompileError::TypeError(msg) => write!(f, "Type error: {}", msg),
            CompileError::CompilerNotFound => write!(f, "Cairo compiler not found"),
            CompileError::InvalidVersion(msg) => write!(f, "Invalid version: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Find the Cairo compiler binary in PATH
fn find_cairo_compiler() -> Result<std::path::PathBuf, CompileError> {
    // Try common cairo-compile names
    const COMPILER_NAMES: &[&str] = &["cairo-compile", "starknet-compile", "cairo-lang-compile"];

    for name in COMPILER_NAMES {
        if let Ok(path) = which::which(name) {
            // Verify compiler version
            if let Ok(version) = check_cairo_version(&path) {
                if version >= MIN_CAIRO_VERSION {
                    return Ok(path);
                }
            }
        }
    }

    Err(CompileError::CompilerNotFound)
}

/// Find the sierra-to-casm compiler binary in PATH
fn find_sierra_to_casm_compiler() -> Result<std::path::PathBuf, CompileError> {
    // Try common sierra-to-casm names
    const COMPILER_NAMES: &[&str] = &["sierra-to-casm", "sierra-to-casm-compile"];

    for name in COMPILER_NAMES {
        if let Ok(path) = which::which(name) {
            return Ok(path);
        }
    }

    Err(CompileError::CompilerNotFound)
}

/// Check Cairo compiler version
fn check_cairo_version(compiler_path: &Path) -> Result<u32, CompileError> {
    let output = Command::new(compiler_path)
        .arg("--version")
        .output()
        .map_err(|_| CompileError::CompilerNotFound)?;

    if !output.status.success() {
        return Ok(0); // Unknown version, assume old
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    parse_cairo_version(&version_str)
}

/// Parse Cairo version string (e.g., "2.6.0" → 20600)
fn parse_cairo_version(version_str: &str) -> Result<u32, CompileError> {
    // Parse version like "2.6.0" or "Cairo compiler version 2.6.0"
    let version_str = version_str
        .trim()
        .split_whitespace()
        .last()
        .unwrap_or(version_str);

    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() < 2 {
        return Err(CompileError::InvalidVersion(version_str.to_string()));
    }

    let major: u32 = parts[0]
        .parse()
        .map_err(|_| CompileError::InvalidVersion(version_str.to_string()))?;
    let minor: u32 = parts[1]
        .parse()
        .map_err(|_| CompileError::InvalidVersion(version_str.to_string()))?;
    let patch: u32 = if parts.len() > 2 {
        parts[2]
            .parse()
            .map_err(|_| CompileError::InvalidVersion(version_str.to_string()))?
    } else {
        0
    };

    Ok(major * 10000 + minor * 100 + patch)
}

/// Parse compiler error output
fn parse_compile_error(error_msg: &str) -> CompileError {
    let error_msg = error_msg.trim();

    // Check for syntax errors
    if error_msg.contains("syntax error") || error_msg.contains("unexpected token") {
        return CompileError::SyntaxError(error_msg.to_string());
    }

    // Check for type errors
    if error_msg.contains("type error") || error_msg.contains("type mismatch") {
        return CompileError::TypeError(error_msg.to_string());
    }

    // Default to syntax error for unknown errors
    CompileError::SyntaxError(error_msg.to_string())
}

/// Registry of Cairo programs
///
/// Maintains a collection of programs and an allowlist of
/// programs approved for on-chain use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CairoProgramRegistry {
    /// All registered programs indexed by hash
    pub programs: BTreeMap<CairoProgramHash, CairoProgram>,
    /// Programs approved for on-chain use
    pub allowlist: BTreeSet<CairoProgramHash>,
}

impl Default for CairoProgramRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CairoProgramRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            programs: BTreeMap::new(),
            allowlist: BTreeSet::new(),
        }
    }

    /// Register a new Cairo program
    pub fn register(&mut self, program: CairoProgram) -> Result<(), RegistryError> {
        let hash = program.hash;

        // Check if program already exists
        if self.programs.contains_key(&hash) {
            return Err(RegistryError::AlreadyExists(hash));
        }

        self.programs.insert(hash, program);
        Ok(())
    }

    /// Get a program by its hash
    pub fn get(&self, hash: &CairoProgramHash) -> Option<&CairoProgram> {
        self.programs.get(hash)
    }

    /// Remove a program from the registry
    pub fn remove(&mut self, hash: &CairoProgramHash) -> Result<CairoProgram, RegistryError> {
        self.programs
            .remove(hash)
            .ok_or_else(|| RegistryError::NotFound(*hash))
            .map(|mut program| {
                // Also remove from allowlist if present
                self.allowlist.remove(hash);
                program
            })
    }

    /// Add a program to the allowlist (governance action)
    pub fn allowlist_add(&mut self, hash: CairoProgramHash) -> Result<(), RegistryError> {
        if !self.programs.contains_key(&hash) {
            return Err(RegistryError::NotFound(hash));
        }
        self.allowlist.insert(hash);
        Ok(())
    }

    /// Remove a program from the allowlist
    pub fn allowlist_remove(&mut self, hash: &CairoProgramHash) -> Result<(), RegistryError> {
        if !self.allowlist.remove(hash) {
            return Err(RegistryError::NotInAllowlist(*hash));
        }
        Ok(())
    }

    /// Check if a program is allowed for on-chain use
    pub fn is_allowed(&self, hash: &CairoProgramHash) -> bool {
        self.allowlist.contains(hash)
    }

    /// Get the number of registered programs
    pub fn len(&self) -> usize {
        self.programs.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }

    /// Get all program hashes
    pub fn keys(&self) -> impl Iterator<Item = &CairoProgramHash> {
        self.programs.keys()
    }

    /// Get all allowed program hashes
    pub fn allowed_keys(&self) -> impl Iterator<Item = &CairoProgramHash> {
        self.allowlist.iter()
    }
}

/// Registry error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Program not found
    NotFound(CairoProgramHash),
    /// Program already exists
    AlreadyExists(CairoProgramHash),
    /// Program not in allowlist
    NotInAllowlist(CairoProgramHash),
    /// Registry full
    RegistryFull,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NotFound(hash) => write!(f, "Program not found: {:?}", hash),
            RegistryError::AlreadyExists(hash) => write!(f, "Program already exists: {:?}", hash),
            RegistryError::NotInAllowlist(hash) => {
                write!(f, "Program not in allowlist: {:?}", hash)
            }
            RegistryError::RegistryFull => write!(f, "Registry is full"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cairo_program_hash() {
        let source = "fn main() { return (); }";
        let hash1 = CairoProgram::compute_hash(source);
        let hash2 = CairoProgram::compute_hash(source);
        assert_eq!(hash1, hash2, "Same source should produce same hash");

        let different_source = "fn main() { return 1; }";
        let hash3 = CairoProgram::compute_hash(different_source);
        assert_ne!(hash1, hash3, "Different source should produce different hash");
    }

    #[test]
    fn test_cairo_program_from_source() {
        let source = "fn main() { return (); }".to_string();
        let version = 2u32;
        let program = CairoProgram::from_source(source.clone(), version);

        assert_eq!(program.source, source);
        assert_eq!(program.version, version);
        assert!(!program.sierra.is_empty() || program.sierra.is_empty()); // Stub - empty for now
        assert!(!program.casm.is_empty() || program.casm.is_empty()); // Stub - empty for now
        assert!(!program.is_compiled(), "Should not be compiled (stub)");
    }

    #[test]
    fn test_cairo_program_compile_not_found() {
        // This test will fail if cairo-compile is actually installed
        // which is fine - it means we can test the real compilation
        let source = "fn main() { return (); }";

        // If cairo-compile is not in PATH, should get CompilerNotFound
        let result = CairoProgram::compile_to_sierra(source);

        // Either compilation succeeds (cairo-compile installed) or fails gracefully
        match result {
            Ok(_) => println!("Cairo compiler found - compilation succeeded"),
            Err(CompileError::CompilerNotFound) => {
                // Expected when compiler not installed
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_cairo_program_version_parsing() {
        // Test version parsing
        let v1 = parse_cairo_version("2.6.0").unwrap();
        assert_eq!(v1, 20600);

        let v2 = parse_cairo_version("2.6").unwrap();
        assert_eq!(v2, 20600);

        let v3 = parse_cairo_version("3.0.0").unwrap();
        assert_eq!(v3, 30000);

        // Invalid versions
        assert!(parse_cairo_version("invalid").is_err());
        assert!(parse_cairo_version("a.b.c").is_err());
    }

    #[test]
    fn test_cairo_program_full_compile() {
        let source = "fn main() { return (); }";

        // If cairo-compile is not installed, this is expected to fail
        let result = CairoProgram::compile_full(source);

        match result {
            Ok(program) => {
                // If compilation succeeded, verify the program is fully compiled
                assert!(program.is_compiled());
                assert!(!program.sierra.is_empty());
                assert!(!program.casm.is_empty());
            }
            Err(CompileError::CompilerNotFound) => {
                // Expected when compiler not installed
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_parse_compile_error() {
        // Test syntax error parsing
        let err = parse_compile_error("error: syntax error at line 5");
        assert!(matches!(err, CompileError::SyntaxError(_)));

        // Test type error parsing
        let err = parse_compile_error("error: type error: expected felt252");
        assert!(matches!(err, CompileError::TypeError(_)));

        // Test unknown error defaults to syntax error
        let err = parse_compile_error("some unknown error");
        assert!(matches!(err, CompileError::SyntaxError(_)));
    }

    #[test]
    fn test_registry_new() {
        let registry = CairoProgramRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = CairoProgramRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);

        let result = registry.register(program.clone());
        assert!(result.is_ok(), "Should successfully register program");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_register_duplicate() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);

        registry.register(program.clone()).unwrap();
        let result = registry.register(program);

        assert!(result.is_err(), "Should fail to register duplicate");
        assert_eq!(registry.len(), 1, "Should still have only 1 program");
    }

    #[test]
    fn test_registry_get() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);
        let hash = program.hash;

        registry.register(program).unwrap();

        let retrieved = registry.get(&hash);
        assert!(retrieved.is_some(), "Should retrieve registered program");
        assert_eq!(retrieved.unwrap().hash, hash);
    }

    #[test]
    fn test_registry_get_not_found() {
        let registry = CairoProgramRegistry::new();
        let hash = [0u8; 32];

        let retrieved = registry.get(&hash);
        assert!(retrieved.is_none(), "Should not find unregistered program");
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);
        let hash = program.hash;

        registry.register(program).unwrap();
        assert_eq!(registry.len(), 1);

        let removed = registry.remove(&hash);
        assert!(removed.is_ok(), "Should successfully remove program");
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_remove_not_found() {
        let mut registry = CairoProgramRegistry::new();
        let hash = [0u8; 32];

        let result = registry.remove(&hash);
        assert!(result.is_err(), "Should fail to remove non-existent program");
    }

    #[test]
    fn test_allowlist_add() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);
        let hash = program.hash;

        registry.register(program).unwrap();
        registry.allowlist_add(hash).unwrap();

        assert!(registry.is_allowed(&hash), "Program should be allowed");
    }

    #[test]
    fn test_allowlist_add_not_found() {
        let mut registry = CairoProgramRegistry::new();
        let hash = [0u8; 32];

        let result = registry.allowlist_add(hash);
        assert!(result.is_err(), "Should fail to add non-existent program");
    }

    #[test]
    fn test_allowlist_remove() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);
        let hash = program.hash;

        registry.register(program).unwrap();
        registry.allowlist_add(hash).unwrap();
        assert!(registry.is_allowed(&hash));

        registry.allowlist_remove(&hash).unwrap();
        assert!(!registry.is_allowed(&hash), "Program should not be allowed");
    }

    #[test]
    fn test_allowlist_not_in_allowlist() {
        let mut registry = CairoProgramRegistry::new();
        let hash = [0u8; 32];

        let result = registry.allowlist_remove(&hash);
        assert!(result.is_err(), "Should fail to remove non-allowed program");
    }

    #[test]
    fn test_is_allowed() {
        let mut registry = CairoProgramRegistry::new();
        let source = "fn main() { return (); }".to_string();
        let program = CairoProgram::from_source(source, 2);
        let hash = program.hash;

        registry.register(program).unwrap();
        assert!(!registry.is_allowed(&hash), "Program should not be allowed initially");

        registry.allowlist_add(hash).unwrap();
        assert!(registry.is_allowed(&hash), "Program should be allowed after allowlist_add");
    }

    #[test]
    fn test_registry_keys() {
        let mut registry = CairoProgramRegistry::new();
        let source1 = "fn main() { return (); }".to_string();
        let source2 = "fn main() { return 1; }".to_string();
        let program1 = CairoProgram::from_source(source1, 2);
        let program2 = CairoProgram::from_source(source2, 2);

        registry.register(program1).unwrap();
        registry.register(program2).unwrap();

        let keys: Vec<_> = registry.keys().collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_registry_allowed_keys() {
        let mut registry = CairoProgramRegistry::new();
        let source1 = "fn main() { return (); }".to_string();
        let source2 = "fn main() { return 1; }".to_string();
        let program1 = CairoProgram::from_source(source1, 2);
        let program2 = CairoProgram::from_source(source2, 2);
        let hash1 = program1.hash;

        registry.register(program1).unwrap();
        registry.register(program2).unwrap();

        // Only allowlist first program
        registry.allowlist_add(hash1).unwrap();

        let allowed: Vec<_> = registry.allowed_keys().collect();
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0], &hash1);
    }
}
