use std::{env, path::PathBuf};

fn main() {
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("missing manifest dir"))
        .join("../desktop-shell/src-tauri/icons/icon.ico");
    println!("cargo:rerun-if-changed={}", icon.display());

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    assert!(
        icon.is_file(),
        "Windows brand icon is missing: {}",
        icon.display()
    );
    let icon = icon
        .to_str()
        .expect("Windows brand icon path must be valid UTF-8");
    tauri_winres::WindowsResource::new()
        .set_icon(icon)
        .set("ProductName", "一龙开发平台")
        .set("FileDescription", "一龙开发平台 Windows 客户端")
        .set("OriginalFilename", "一龙开发平台.exe")
        .compile_for(&["elon-pc-node"])
        .expect("failed to embed the Windows brand icon");
    tauri_winres::WindowsResource::new()
        .set_icon(icon)
        .set("ProductName", "一龙开发平台")
        .set("FileDescription", "一龙开发平台 Windows 安装程序")
        .set("OriginalFilename", "Elon-Windows-Setup.exe")
        .compile_for(&["elon-node-installer"])
        .expect("failed to embed the Windows installer brand icon");
}
