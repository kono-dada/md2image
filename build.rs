use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let asset_root = manifest_dir.join("vendor").join("katex");
    println!("cargo:rerun-if-changed={}", asset_root.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let generated = out_dir.join("katex_assets.rs");
    let mut assets = Vec::new();
    collect_assets(&asset_root, &asset_root, &mut assets);
    assets.sort();

    let mut source = String::from("pub const KATEX_ASSETS: &[(&str, &[u8])] = &[\n");
    for relative in assets {
        source.push_str(&format!(
            "    ({relative:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/vendor/katex/{relative}\"))),\n"
        ));
    }
    source.push_str("];\n");

    fs::write(generated, source).expect("write generated asset manifest");
}

fn collect_assets(root: &Path, dir: &Path, assets: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read asset directory {}: {error}", dir.display())
    });

    for entry in entries {
        let entry = entry.expect("read asset entry");
        let path = entry.path();
        if path.is_dir() {
            collect_assets(root, &path, assets);
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("asset should be under root")
            .to_string_lossy()
            .replace('\\', "/");
        assets.push(relative);
    }
}
