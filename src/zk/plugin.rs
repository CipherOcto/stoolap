// src/zk/plugin.rs
// STWO plugin loading and verification

use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::path::Path;

/// Result of a verification operation from the plugin
#[repr(C)]
pub struct StarkVerifyResult {
    pub success: bool,
    pub error_message: *const c_char,
}

/// Errors when plugin is not available or fails
#[derive(Debug, Clone, PartialEq)]
pub enum PluginError {
    /// Plugin file not found
    NotFound,
    /// Failed to load the plugin library
    LoadFailed(String),
    /// Required symbol not found in plugin
    SymbolNotFound,
    /// Verification failed
    VerificationFailed(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::NotFound => write!(
                f,
                "STWO plugin not found. Set STOOLAP_STWO_PLUGIN environment variable \
                or build the stwo-plugin crate from https://github.com/mmacedoeu/stwo-plugin"
            ),
            PluginError::LoadFailed(msg) => write!(f, "Failed to load STWO plugin: {}", msg),
            PluginError::SymbolNotFound => write!(f, "STWO plugin missing required symbols"),
            PluginError::VerificationFailed(msg) => write!(f, "STWO verification failed: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

/// STWO Plugin wrapper
#[derive(Debug)]
pub struct STWOPlugin {
    #[allow(dead_code)]
    lib: Library,
}

impl STWOPlugin {
    /// Load the plugin from a .so file
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        let lib =
            unsafe { Library::new(path) }.map_err(|e| PluginError::LoadFailed(e.to_string()))?;
        Ok(Self { lib })
    }

    /// Verify a proof using the loaded plugin
    pub fn verify(&self, proof: &[u8]) -> Result<bool, PluginError> {
        type VerifyFn = unsafe extern "C" fn(*const u8, usize) -> StarkVerifyResult;

        let func: Symbol<VerifyFn> = unsafe {
            self.lib
                .get(b"stark_verify_proof")
                .map_err(|_| PluginError::SymbolNotFound)?
        };

        let result = unsafe { func(proof.as_ptr(), proof.len()) };

        if result.success {
            Ok(true)
        } else {
            let msg = if result.error_message.is_null() {
                "Unknown error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(result.error_message) }
                    .to_string_lossy()
                    .into_owned()
            };
            Err(PluginError::VerificationFailed(msg))
        }
    }

    /// Get plugin version
    pub fn version(&self) -> Result<String, PluginError> {
        type VersionFn = extern "C" fn() -> *const c_char;

        let func: Symbol<VersionFn> = unsafe {
            self.lib
                .get(b"stark_plugin_version")
                .map_err(|_| PluginError::SymbolNotFound)?
        };

        let version_cstr = unsafe { std::ffi::CStr::from_ptr(func()) };
        Ok(version_cstr.to_string_lossy().into_owned())
    }
}

/// Try to load the STWO plugin
///
/// Search order:
/// 1. Environment variable `STOOLAP_STWO_PLUGIN`
/// 2. Default path `./stwo-plugin/target/release/libstwo_plugin.so`
pub fn load_plugin() -> Result<STWOPlugin, PluginError> {
    // 1. Check environment variable
    if let Ok(path) = std::env::var("STOOLAP_STWO_PLUGIN") {
        return STWOPlugin::load(Path::new(&path));
    }

    // 2. Check default path (inside stoolap_chain)
    let default_path = Path::new(".")
        .join("stwo-plugin")
        .join("target")
        .join("release")
        .join("libstwo_plugin.so");

    if default_path.exists() {
        return STWOPlugin::load(&default_path);
    }

    // Also check current directory for development
    let local_path = Path::new("libstwo_plugin.so");
    if local_path.exists() {
        return STWOPlugin::load(local_path);
    }

    // 3. Not found
    Err(PluginError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_plugin_not_found() {
        // This should fail since we haven't built the plugin yet in tests
        let result = load_plugin();
        // The plugin may or may not exist depending on build
        println!("Plugin load result: {:?}", result);
    }
}
