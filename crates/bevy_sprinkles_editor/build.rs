use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let examples_dir = Path::new(&manifest_dir).join("src/assets/examples");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    // rerun when examples are added/removed/modified
    println!("cargo:rerun-if-changed={}", examples_dir.display());

    let mut ron_stems: Vec<String> = Vec::new();
    let mut jpg_stems: Vec<String> = Vec::new();

    let entries = fs::read_dir(&examples_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", examples_dir.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        let target = match ext {
            "ron" => &mut ron_stems,
            "jpg" => &mut jpg_stems,
            _ => continue,
        };

        println!("cargo:rerun-if-changed={}", path.display());
        target.push(stem.to_string());
    }

    ron_stems.sort();
    jpg_stems.sort();

    let mut bundled = String::new();
    bundled.push_str("const BUNDLED_EXAMPLES: &[(&str, &str)] = &[\n");
    for stem in &ron_stems {
        let abs_path = examples_dir.join(format!("{stem}.ron"));
        bundled.push_str(&format!(
            "    (\"{stem}.ron\", include_str!(\"{}\")),\n",
            abs_path.display()
        ));
    }
    bundled.push_str("];\n");

    fs::write(out_path.join("bundled_examples.rs"), bundled).unwrap();

    let thumbs_dest = out_path.join("assets/examples");
    fs::create_dir_all(&thumbs_dest).unwrap();

    let mut thumbnails = String::from("{\n");
    for stem in &jpg_stems {
        let src = examples_dir.join(format!("{stem}.jpg"));
        let dst = thumbs_dest.join(format!("{stem}.jpg"));
        fs::copy(&src, &dst).unwrap();

        thumbnails.push_str(&format!(
            "    embedded_asset!(app, \"out\", \"assets/examples/{stem}.jpg\");\n"
        ));
    }
    thumbnails.push_str("}");

    fs::write(out_path.join("embed_thumbnails.rs"), thumbnails).unwrap();
}
