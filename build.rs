fn main() {
    if cfg!(target_os = "macos") {
        let path = std::fs::canonicalize("./vendored/camera/mac");
        println!("cargo:rustc-link-search=./vendored/camera/mac");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        let path = std::fs::canonicalize("./vendored/camera/linux/x64");
        println!("cargo:rustc-link-search=./vendored/camera/linux/x64");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86") {
        let path = std::fs::canonicalize("./vendored/camera/linux/x86");
        println!("cargo:rustc-link-search=./vendored/camera/linux/x86");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "arm") {
        let path = std::fs::canonicalize("./vendored/camera/linux/armv7");
        println!("cargo:rustc-link-search=./vendored/camera/linux/armv7");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        let path = std::fs::canonicalize("./vendored/camera/linux/armv8");
        println!("cargo:rustc-link-search=./vendored/camera/linux/armv8");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "windows") {
        let path = std::fs::canonicalize("./vendored/camera/windows/x64");
        println!("cargo:rustc-link-search=./vendored/camera/windows/x64");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    println!("cargo:rustc-link-lib=ASICamera2");
}
