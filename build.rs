use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn escape_rust_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=binaries");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binaries_dir = Path::new(&manifest_dir).join("binaries");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is required"));
    let generated_path = out_dir.join("embedded_binaries.rs");

    let mut files = Vec::new();
    if let Err(e) = collect_files(&binaries_dir, &mut files) {
        panic!("failed to scan binaries directory: {e}");
    }
    files.sort();

    let mut generated = String::new();
    generated.push_str("const EMBEDDED_BINARIES: &[EmbeddedBinary] = &[\n");

    for file in files {
        let filename = file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let escaped_name = escape_rust_string(filename);
        let abs = file.canonicalize().unwrap_or(file);
        let escaped_path = escape_rust_string(&abs.to_string_lossy());

        generated.push_str("    EmbeddedBinary {\n");
        generated.push_str(&format!("        name: \"{}\",\n", escaped_name));
        generated.push_str(&format!("        bytes: include_bytes!(\"{}\"),\n", escaped_path));
        generated.push_str("    },\n");
    }

    generated.push_str("];\n");
    fs::write(generated_path, generated).expect("failed to write embedded_binaries.rs");
}
