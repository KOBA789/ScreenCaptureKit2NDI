use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=/Library/NDI SDK for Apple/lib/macOS");
    println!("cargo:rustc-link-lib=ndi");
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .size_t_is_usize(true)
        .header("wrapper.h")
        .trust_clang_mangling(false)
        .derive_default(true)
        .clang_arg("-I/Library/NDI SDK for Apple/include")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        //.parse_callbacks(Box::new(TrimPrefixCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct TrimPrefixCallbacks;
impl bindgen::callbacks::ParseCallbacks for TrimPrefixCallbacks {
    fn item_name(&self, original_item_name: &str) -> Option<String> {
        original_item_name
            .strip_prefix("NDIlib_")
            .map(ToOwned::to_owned)
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        let enum_name = enum_name?.strip_prefix("enum ")?.strip_suffix("_e")?;
        let variant_name = original_variant_name.strip_prefix(enum_name)?.strip_prefix('_')?;
        Some(variant_name.to_string())
    }
}
