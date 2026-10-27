fn main()
{
    // Re-embed the exe icon whenever the icon files change
    println!("cargo:rerun-if-changed=icons");

    // Which webview may call which command is set in capabilities/youtube.json and capabilities/tabbar.json
    let manifest = tauri_build::AppManifest::new().commands(&[
        "open_external",
        "new_tab",
        "close_tab",
        "close_current",
        "activate_tab",
        "cycle_tab",
        "set_fullscreen",
        "start_drag",
        "set_mini",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
