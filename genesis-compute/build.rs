fn main() {
    // §1.3. Отсечка для Mock-GPU
    if std::env::var("CARGO_FEATURE_MOCK_GPU").is_ok() {
        return;
    }

    println!("cargo:rerun-if-changed=src/cuda/");
    println!("cargo:rerun-if-changed=src/amd/");

    if cfg!(feature = "amd") {
        cc::Build::new()
            .compiler("hipcc")
            .file("src/amd/bindings.hip")
            .file("src/amd/physics.hip")
            .flag("-O3")
            .flag("--offload-arch=gfx803") 
            .compile("genesis_amd");

        if cfg!(target_os = "linux") {
            println!("cargo:rustc-link-search=native=/opt/rocm/lib");
        }
        println!("cargo:rustc-link-lib=dylib=amdhip64");
    } else {
        cc::Build::new()
            .cuda(true)
            .flag("-arch=sm_61") 
            .flag("-O3")
            .flag("-w")
            .file("src/cuda/bindings.cu")
            .file("src/cuda/physics.cu")
            .compile("genesis_cuda");

        println!("cargo:rustc-link-lib=dylib=cudart");
    }
}
