use std::path::PathBuf;
use std::process::Command;

fn sdk_path() -> Result<String, std::io::Error> {
    let sdk = "macosx";
    let output = Command::new("xcrun")
        .args(&["--sdk", sdk, "--show-sdk-path"])
        .output()?
        .stdout;
    let prefix_str = std::str::from_utf8(&output).expect("invalid output from `xcrun`");
    Ok(prefix_str.trim_end().to_string())
}

fn main() {
    println!("cargo:rustc-link-lib=framework=CoreMedia");
    println!("cargo:rustc-link-lib=framework=CoreVideo");
    println!("cargo:rerun-if-changed=wrapper.h");

    let sdk_path = sdk_path().expect("Failed to get SDK path");
    let bindings = bindgen::Builder::default()
        .size_t_is_usize(true)
        .header("wrapper.h")
        .trust_clang_mangling(false)
        .derive_default(true)
        .clang_args(&["-isysroot", &sdk_path])
        .allowlist_type("k?C[MV].*")
        .allowlist_function("k?C[MV].*")
        .allowlist_var("k?C[MV].*")
        .allowlist_var("kCG.*")
        .allowlist_recursively(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
