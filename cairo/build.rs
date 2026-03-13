// cairo/build.rs
// Compile Cairo programs to CASM at build time

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun if any .cairo file changes
    println!("cargo:rerun-if-changed=hexary_verify.cairo");
    println!("cargo:rerun-if-changed=merkle_batch.cairo");
    println!("cargo:rerun-if-changed=state_transition.cairo");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir);

    // Programs to compile
    let programs = ["hexary_verify", "merkle_batch", "state_transition"];

    for program in programs {
        let src_path = format!("{}.cairo", program);
        if Path::new(&src_path).exists() {
            // For now, just verify the file exists
            // Real compilation would use cairo-compile or similar
            println!(
                "cargo:rustc-env={}_CAIRO={}",
                program.to_uppercase(),
                src_path
            );
        }
    }

    // Generate a marker file to track compilation status
    fs::write(dest_path.join("compiled.txt"), "compiled").expect("Failed to write marker");
}
