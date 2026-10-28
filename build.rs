// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes

// Generates assets/icon.ico from the same pixel art as the runtime window
// and tray icons (src/icon_art.rs) and embeds it as the .exe's PE resource
// icon, so Start menu / taskbar pins show the app icon instead of a blank one.
// Release builds also get a requireAdministrator manifest: GPU utilisation,
// temps and ETW FPS counting all need an elevated token.

include!("src/icon_art.rs");

const ADMIN_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

fn generate_icon(path: &std::path::Path) {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 32, 48, 64, 128, 256] {
        let rgba = draw_icon_rgba(size);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        icon_dir.add_entry(ico::IconDirEntry::encode(&image).expect("encode icon frame"));
    }
    let file = std::fs::File::create(path).expect("create icon.ico");
    icon_dir.write(file).expect("write icon.ico");
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let icon_path = std::path::Path::new(&out_dir).join("icon.ico");
    generate_icon(&icon_path);

    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon_path.to_str().expect("icon path is valid UTF-8"));
    // Release only: winresource links this into every binary, including the `cargo test`
    // harness, which would then refuse to start unelevated (os error 740).
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        res.set_manifest(ADMIN_MANIFEST);
    }
    res.compile().expect("failed to embed Windows icon resource");
}
