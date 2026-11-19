use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }

    // GPUI loads icon resource 1 for the Windows window class. Reuse the
    // production ICO so Explorer, Alt+Tab and the window share its identity.
    let icon =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/icons/icon.ico");
    assert!(icon.is_file(), "Production DiToVR icon is required");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource = out.join("ditovr-icon.rc");
    let compiled = out.join("ditovr-icon.res");
    fs::write(
        &resource,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("Could not write icon resource");
    let status = Command::new("rc.exe")
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(&resource)
        .status()
        .expect("Windows SDK rc.exe is required; run build.ps1");
    assert!(status.success(), "Could not compile DiToVR icon resource");
    println!("cargo:rustc-link-arg-bins={}", compiled.display());
}
