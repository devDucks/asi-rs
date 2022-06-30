fn main() {
    let mut os: &str = "";
    let mut arch: Option<&str> = None;

    if cfg!(target_os = "macos") {
        os = "mac";
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        os = "linux";
        arch = Some("x64");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "x86") {
        os = "linux";
        arch = Some("x86");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "arm") {
        os = "linux";
        arch = Some("armv7");
    };

    if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        os = "linux";
        arch = Some("armv8");
    };

    if cfg!(target_os = "windows") {
        os = "windows";
        arch = Some("x64");
    };

    let paths = [
        {
            if let Some(a) = arch {
                std::fs::canonicalize(format!("./vendored/camera/{}/{}", os, a))
            } else {
                std::fs::canonicalize(format!("./vendored/camera/{}", os))
            }
        },
        {
            if let Some(a) = arch {
                std::fs::canonicalize(format!("./vendored/efw/{}/{}", os, a))
            } else {
                std::fs::canonicalize(format!("./vendored/efw/{}", os))
            }
        },
    ];

    for path in paths {
        if let Some(s_path) = path.unwrap().as_os_str().to_str() {
            println!("cargo:rustc-link-search={}", &s_path);
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", &s_path);
        }
    }

    println!("cargo:rustc-link-lib=ASICamera2");
    println!("cargo:rustc-link-lib=EFWFilter");
}
