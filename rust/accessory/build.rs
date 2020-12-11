use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let baltech_dir = crate_dir.join("baltech");
    if !baltech_dir.exists() {
        let res = std::process::Command::new("/bin/bash")
            .args(&[
                "-c",
                "wget -O baltech_sdk.zip https://axartifacts.blob.core.windows.net/artifacts/00_CI/6003_baltech_sdk_3_13_00.zip && \
                unzip baltech_sdk.zip -d baltech && patch -p0 < baltech_sdk.patch",
            ])
            .current_dir(crate_dir)
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        assert!(res.status.success());
    }

    // brp_lib is linked dynamically
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let brp_lib_out = out_dir.join("brp_lib");
    let _ = std::fs::create_dir_all(brp_lib_out.clone());
    let brp_lib_out_dir = cmake::Config::new("baltech/brp_lib")
        .no_build_target(true)
        // Ignore errors on release build
        .cflag("-w")
        .out_dir(brp_lib_out.clone())
        .build();

    // Copy generated lib to out_dir for convenience
    let lib_postfix = if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        "dll"
    } else {
        "so"
    };
    std::fs::copy(
        brp_lib_out_dir.join(format!("build/libbrp_lib.{}", lib_postfix)),
        out_dir.join(format!("../../../libbrp_lib.{}", lib_postfix)),
    )
    .unwrap();

    println!("cargo:rustc-link-search=native={}", brp_lib_out.join("build").display());
    // brp_lib is linked dynamically
    println!("cargo:rustc-link-lib=dylib=brp_lib");

    let baltech_lib_out = out_dir.join("baltech_lib");
    let _ = std::fs::create_dir_all(baltech_lib_out.clone());
    cmake::Config::new("baltech/baltech_api/c")
        .no_build_target(true)
        // Ignore errors on release build
        .cflag("-w")
        .out_dir(baltech_lib_out.clone())
        .build();
    // baltech lib is linked statically
    println!(
        "cargo:rustc-link-search=native={}",
        baltech_lib_out.join("build").display()
    );
    println!("cargo:rustc-link-lib=static=baltech_api");

    // brp_lib is included in baltech_api
    let bindings_baltech_api = bindgen::Builder::default()
        .header("baltech/baltech_api/c/baltech_api.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .derive_default(true)
        .clang_arg("-Ibaltech/brp_lib/inc")
        .whitelist_function("brp_.*")
        .whitelist_function("BRP_.*")
        .whitelist_type("brp_.*")
        .whitelist_type("BRP_.*")
        .whitelist_var("brp_.*")
        .whitelist_var("BRP_.*")
        .default_enum_style(bindgen::EnumVariation::NewType { is_bitfield: false })
        .generate()
        .expect("Unable to generate bindings");
    bindings_baltech_api
        .write_to_file(out_dir.join("bindings_baltech_api.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=Cargo.lock");
}
