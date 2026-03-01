# Mission: Cairo Compiler Integration

## Status
Open

## RFC
RFC-0201: STWO and Cairo Integration for Zero-Knowledge Proofs

## Acceptance Criteria
- [ ] Add `cairo-lang-compiler` as build dependency
- [ ] Implement `CairoProgram::compile_to_sierra()` with actual Cairo compiler
- [ ] Implement `CairoProgram::compile_to_casm()` with Sierra→CASM compilation
- [ ] Add error handling for compilation failures
- [ ] Add integration tests with sample Cairo program
- [ ] Add compiler version detection

## Dependencies
- Mission 0201-02 (Cairo Program Types)

## Enables
- Mission 0201-04 (Core Cairo Programs)

## Implementation Notes

**Files to Modify:**
- `Cargo.toml` - Add Cairo compiler dependency
- `src/zk/cairo.rs` - Implement compilation methods

**Cairo Compiler Dependency:**
```toml
[build-dependencies]
cairo-lang-compiler = "2.6"
```

**Compilation Pipeline:**
```rust
impl CairoProgram {
    pub fn compile_to_sierra(source: &str) -> Result<Vec<u8>, CompileError> {
        // Call Cairo compiler to generate Sierra IR
        let compiler = CairoCompiler::new();
        compiler.compile_to_sierra(source)
    }

    pub fn compile_to_casm(sierra: &[u8]) -> Result<Vec<u8>, CompileError> {
        // Compile Sierra to CASM
        let compiler = CairoCompiler::new();
        compiler.sierra_to_casm(sierra)
    }
}
```

**Error Handling:**
```rust
#[derive(Debug)]
pub enum CompileError {
    SyntaxError(String),
    TypeError(String),
    CompilerNotFound,
    InvalidVersion(String),
}
```

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
