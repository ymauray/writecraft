//! Windows only: embed the app icon and version info (VERSIONINFO) into `wordcraft.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `WORDCRAFT_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/writecraft-icon/writecraft.ico");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    // WriteCraft: its own name and icon (packaging/writecraft/windows.ps1 ships it as writecraft.exe).
    res.set_icon("../../assets/writecraft-icon/writecraft.ico")
        .set("ProductName", "WriteCraft")
        .set("FileDescription", "WriteCraft word processor")
        .set("CompanyName", "WriteCraft contributors")
        .set("LegalCopyright", "Copyright (c) the WriteCraft and WordCraft authors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "writecraft.exe")
        .set("InternalName", "writecraft");
    if let Err(e) = res.compile() {
        if std::env::var_os("WORDCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=wordcraft.exe built without icon/version resources: {e}");
    }
}
