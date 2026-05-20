use std::{
    fs,
    io::Write,
    path::Path,
};

pub fn collect() -> Vec<String> {
    let mut rs_files: Vec<String> = Vec::new();
    let mut ui_files: Vec<String> = Vec::new();

    collect_by_ext(Path::new("src"), "rs", &mut rs_files);
    collect_by_ext(Path::new("resources/ui"), "ui", &mut ui_files);

    rs_files.sort();
    ui_files.sort();

    let mut files = rs_files;
    files.extend(ui_files);
    files
}

pub fn write(files: &[String]) {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=resources/ui");

    let content = format!("{}\n", files.join("\n"));
    let existing = fs::read_to_string("po/POTFILES").unwrap_or_default();

    if existing == content {
        return;
    }

    let mut out = fs::File::create("po/POTFILES").expect("Failed to create po/POTFILES");
    out.write_all(content.as_bytes())
        .expect("Failed to write po/POTFILES");

    println!("cargo:warning=po/POTFILES updated");
}

fn collect_by_ext(dir: &Path, ext: &str, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_by_ext(&path, ext, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            // Normalise to forward-slash so POTFILES is consistent across OSes.
            out.push(path.to_str().unwrap().replace('\\', "/"));
        }
    }
}
