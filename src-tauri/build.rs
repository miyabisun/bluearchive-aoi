//! Builds the files under `data/` into the program (`embedded::FILES`), so the
//! released exe needs nothing beside it, and runs Tauri's build step.

use std::{fmt::Write, path::Path};

fn files(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(&path, base, out);
        } else {
            let name = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push(name);
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("data");
    let mut names = Vec::new();
    files(&data, &data, &mut names);
    let mut code = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for name in &names {
        let path = data.join(name).to_string_lossy().replace('\\', "/");
        writeln!(code, "    ({name:?}, include_bytes!({path:?})),").unwrap();
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(out, code).unwrap();
    println!("cargo:rerun-if-changed=data");
    tauri_build::build();
}
