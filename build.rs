// build.rs
use std::process::Command;

fn main() {
    let frontend = std::path::Path::new("frontend");
    if !frontend.exists() {
        return; // no frontend in this checkout
    }

    // Tell cargo to re-run if frontend sources change
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.js");

    // Install dependencies if node_modules missing
    if !frontend.join("node_modules").exists() {
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(frontend)
            .status()
            .expect("npm install failed");
        assert!(status.success(), "npm install failed");
    }

    // Build
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(frontend)
        .status()
        .expect("npm run build failed");
    assert!(status.success(), "frontend build failed");
}
