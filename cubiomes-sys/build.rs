use std::{collections::HashSet, env, path::PathBuf, process::Command};

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

#[derive(Debug)]
struct DeriveMacros(Vec<String>);

impl bindgen::callbacks::ParseCallbacks for DeriveMacros {
    fn add_derives(&self, _info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        self.0.clone()
    }
}

fn main() {
    if !Command::new("make")
        .arg("-C")
        .arg("cubiomes/")
        .status()
        .expect("Failed to build cubiomes")
        .success()
    {
        panic!("Make did not return 0")
    }

    if !Command::new("mv")
        .arg("cubiomes/libcubiomes.a")
        .arg(env::var("OUT_DIR").unwrap())
        .status()
        .expect("Failed to move libcubiomes")
        .success()
    {
        panic!("mv did not return 0")
    }

    println!("cargo:rustc-link-search={}/", env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-lib=static=cubiomes");

    let ignored_macros = IgnoreMacros(
        vec![
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
        ]
        .into_iter()
        .collect(),
    );

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(ignored_macros))
        .newtype_enum(".*")
        .layout_tests(true)
        .generate()
        .expect("Unable to generate binding for cubiomes");

    // Generates rustified enums for use in a wrapper library
    let biome_enum_bindings = bindgen::Builder::default()
        .header("cubiomes/biomes.h")
        .parse_callbacks(Box::new(DeriveMacros(vec![
            "FromPrimitive".into(),
            "ToPrimitive".into(),
        ])))
        .blocklist_function(".*") //Blocks all functions, as we are only intrested in the enums
        .rustified_non_exhaustive_enum(".*")
        .generate()
        .expect("Unable to generate rustified enums for cubiomes");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");

    biome_enum_bindings
        .write_to_file(out_path.join("biome_enums.rs"))
        .expect("Couldn't write biome enums");
}
