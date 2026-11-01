use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from("assets/phosphor");
    let mut files = Vec::new();
    for weight in ["thin", "light", "regular", "bold", "fill", "duotone"] {
        let dir = root.join(weight);
        println!("cargo:rerun-if-changed={}", dir.display());
        for entry in fs::read_dir(dir).expect("read Phosphor assets") {
            let path = entry.expect("read Phosphor entry").path();
            if path.extension().is_some_and(|ext| ext == "svg") {
                files.push(path);
            }
        }
    }
    files.sort();

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut data = Vec::new();
    let mut index = String::from("&[\n");
    for path in files {
        let key = path
            .strip_prefix(&root)
            .expect("asset under root")
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(&path).expect("read Phosphor SVG");
        let offset = data.len();
        let len = bytes.len();
        data.extend_from_slice(&bytes);
        index.push_str(&format!("    ({key:?}, {offset}, {len}),\n"));
    }
    index.push_str("]");
    fs::write(out.join("phosphor.bin"), data).expect("write packed Phosphor assets");
    fs::write(out.join("phosphor_index.rs"), index).expect("write Phosphor index");
}
