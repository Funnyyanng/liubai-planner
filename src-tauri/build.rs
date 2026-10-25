fn main() {
    let manifests: Vec<planner_contracts::ModuleManifest> =
        serde_json::from_str(include_str!("../modules.json")).expect("Invalid module manifests");
    planner_framework::registry::validate(&manifests)
        .expect("Module dependency / contract validation failed");
    println!("cargo:rerun-if-changed=../modules.json");
    tauri_build::build();
}
