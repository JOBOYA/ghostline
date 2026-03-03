use std::process::Command;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let viewer_dir = format!("{}/../../viewer", manifest_dir);
    let dist_dir = format!("{}/dist", viewer_dir);

    // Tell cargo to re-run if viewer source changes
    println!("cargo:rerun-if-changed={}/src", viewer_dir);
    println!("cargo:rerun-if-changed={}/package.json", viewer_dir);

    // Skip viewer build if explicitly disabled
    if std::env::var("GHOSTLINE_SKIP_VIEWER_BUILD").is_ok() {
        println!("cargo:warning=Skipping viewer build (GHOSTLINE_SKIP_VIEWER_BUILD set)");
        return;
    }

    // Only build if dist/ doesn't exist
    if !std::path::Path::new(&dist_dir).exists() {
        println!("cargo:warning=Building viewer...");
        let status = Command::new("npm")
            .args(["run", "build"])
            .current_dir(&viewer_dir)
            .status()
            .expect("failed to run npm — is Node.js installed?");

        if !status.success() {
            panic!("Viewer build failed");
        }
    }
}
