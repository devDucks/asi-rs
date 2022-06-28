fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=./lib/mac");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/mac");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        println!("cargo:rustc-link-search=./lib/linux/x64");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/linux/x64");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86") {
        println!("cargo:rustc-link-search=./lib/linux/x86");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/linux/x86");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "arm") {
        println!("cargo:rustc-link-search=./lib/linux/armv7");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/linux/armv7");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        println!("cargo:rustc-link-search=./lib/linux/armv8");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/linux/armv8");
    };

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=./lib/windows/x64");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/windows/x64");
    };

    println!("cargo:rustc-link-lib=ASICamera2");
}
