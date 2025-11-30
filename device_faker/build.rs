fn main() {
    // 仅在 Android 目标平台编译 C++ atexit 实现
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        cc::Build::new()
            .file("src/atexit.cpp")
            .cpp(true)
            .flag("-std=c++17")
            .flag("-fno-exceptions")
            .flag("-fno-rtti")
            .warnings(false)
            .compile("local_atexit");

        println!("cargo:rerun-if-changed=src/atexit.cpp");
    }
}
