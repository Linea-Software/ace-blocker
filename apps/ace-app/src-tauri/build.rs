//use tauri_typegen::BuildSystem;

fn main() {
    // tonic_prost_build::configure()
    //     .build_server(false)
    //     // Apply to all structs/messages
    //     .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    //     .message_attribute(".", "#[serde(rename_all = \"camelCase\")]")
    //     .enum_attribute(".", "#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]")
    //     .compile_protos(&["proto/ace_engine_api.proto"], &["proto"])
    //     .expect("Couldn't compile protos");

    // BuildSystem::generate_at_build_time().expect("Failed to generate TypeScript bindings");
    // Force Tauri's native resource step to rerun when icon assets change.
    // Otherwise incremental builds can keep the previous icon in ace-app.exe.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/128x128.png");
    tauri_build::build()
}
