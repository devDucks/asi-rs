fn main() {
    if cfg!(target_os = "macos") {
        let path = std::fs::canonicalize("./lib/mac");
        println!("cargo:rustc-link-search=./lib/mac");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        let path = std::fs::canonicalize("./lib/linux/x64");
        println!("cargo:rustc-link-search=./lib/linux/x64");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86") {
        let path = std::fs::canonicalize("./lib/linux/x86");
        println!("cargo:rustc-link-search=./lib/linux/x86");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "arm") {
        let path = std::fs::canonicalize("./lib/linux/armv7");
        println!("cargo:rustc-link-search=./lib/linux/armv7");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        let path = std::fs::canonicalize("./lib/linux/armv8");
        println!("cargo:rustc-link-search=./lib/linux/armv8");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    if cfg!(target_os = "windows") {
        let path = std::fs::canonicalize("./lib/windows/x64");
        println!("cargo:rustc-link-search=./lib/windows/x64");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            path.unwrap().display()
        );
    };

    println!("cargo:rustc-link-lib=ASICamera2");
}
