//! Merge Blade stock WGSL with geofront overlays into OUT_DIR for WASM embed.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/shaders");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("merged-shaders");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("create merged-shaders");

    let stock = find_blade_render_code();
    copy_wgsl(&stock, &out);

    let overlay = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest"))
        .join("assets/shaders");
    if overlay.is_dir() {
        copy_wgsl(&overlay, &out);
    }
}

fn find_blade_render_code() -> PathBuf {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("manifest");
    let mut cmd = Command::new(&cargo);
    cmd.args(["metadata", "--format-version", "1"]);
    if let Ok(target) = env::var("TARGET") {
        cmd.args(["--filter-platform", &target]);
    }
    let output = cmd
        .current_dir(&manifest_dir)
        .output()
        .expect("cargo metadata");
    if !output.status.success() {
        panic!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata");
    for pkg in meta["packages"].as_array().expect("packages") {
        if pkg["name"].as_str() == Some("blade-render") {
            let manifest = pkg["manifest_path"].as_str().expect("manifest_path");
            let code = Path::new(manifest).parent().unwrap().join("code");
            if code.is_dir() {
                return code;
            }
            panic!("blade-render code/ missing at {}", code.display());
        }
    }
    panic!("blade-render not found in cargo metadata (is the git dep fetched?)");
}

fn copy_wgsl(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap_or_else(|e| panic!("read {}: {e}", from.display())) {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        let dest = to.join(entry.file_name());
        fs::copy(&path, &dest).unwrap_or_else(|e| {
            panic!("copy {} -> {}: {e}", path.display(), dest.display())
        });
    }
}
