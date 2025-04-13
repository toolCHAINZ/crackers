use std::process::Command;

fn main() {
    // Run the Python script to get the venv's lib directory
    let output = Command::new("python3")
        .arg("find_venv_library_path.py")
        .output()
        .expect("Failed to execute Python script");

    if !output.status.success() {
        panic!("cargo:warning=Could not find python's z3, this wheel is unlikely to work.");
    } else {
        let venv_lib = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let z3_python_lib = std::path::Path::new(&venv_lib).join("z3").join("lib");
        println!("cargo:rustc-link-search=native={}", z3_python_lib.display());
        unsafe {
            std::env::set_var("Z3_PATH", z3_python_lib);
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=Z3_PATH")
}
