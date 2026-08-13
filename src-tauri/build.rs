fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        cc::Build::new()
            .cpp(true)
            .file("native/nvapi_bridge.cpp")
            .include("native")
            .include("vendor/nvapi")
            .flag_if_supported("/std:c++17")
            .compile("game_passport_nvapi_bridge");
        println!("cargo:rustc-link-search=native={manifest}/vendor/nvapi/amd64");
        println!("cargo:rustc-link-lib=static=nvapi64");
        println!("cargo:rerun-if-changed=native/nvapi_bridge.cpp");
        println!("cargo:rerun-if-changed=native/nvapi_bridge.h");
    }
    tauri_build::build()
}
