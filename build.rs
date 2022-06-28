fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=./lib/mac");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/mac");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        println!("cargo:rustc-link-search=./lib/x64");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/x64");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86") {
        println!("cargo:rustc-link-search=./lib/x86");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/x86");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "arm") {
        println!("cargo:rustc-link-search=./lib/armv7");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/armv7");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        println!("cargo:rustc-link-search=./lib/armv8");
        println!("cargo:rustc-link-arg=-Wl,-rpath,./lib/armv8");
    };

    if cfg!(target_os = "windows") {
        todo!("TBI");
    };

    println!("cargo:rustc-link-lib=ASICamera2");
}
