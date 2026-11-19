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
    let status = Command::new(find_rc())
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(&resource)
        .status()
        .expect("Windows SDK rc.exe is required");
    assert!(status.success(), "Could not compile DiToVR icon resource");
    println!("cargo:rustc-link-arg-bins={}", compiled.display());
}

/// PATHにrc.exeがなければ(CIなど)、Windows SDKの最新版から探す。
fn find_rc() -> PathBuf {
    let in_path = env::var_os("PATH")
        .map(|p| env::split_paths(&p).any(|d| d.join("rc.exe").is_file()))
        .unwrap_or(false);
    if in_path {
        return PathBuf::from("rc.exe");
    }
    let kits = PathBuf::from(
        env::var_os("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".into()),
    )
    .join("Windows Kits")
    .join("10")
    .join("bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(&kits)
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|v| v.join("x64").join("rc.exe"))
        .find(|p| p.is_file())
        .expect("Windows SDK rc.exe was not found")
}
