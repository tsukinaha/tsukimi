#![allow(dead_code, unused_variables)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_RESOURCE_PREFIX: &str = "/io/github/mutsumi";

fn main() {
    // on_build();
    //
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "mutsumi.gresource",
    );
}

fn on_build() {
    let project_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"));

    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("resources/icons").display()
    );

    let blueprint_inputs = discover_files_with_extension(&project_root.join("src"), "blp");
    let icon_inputs = discover_icon_files(&project_root.join("resources/icons"));

    check_duplicate_ui_outputs(&project_root, &blueprint_inputs);
    cleanup_generated_ui_files(&project_root, &blueprint_inputs);

    for input in &blueprint_inputs {
        compile_blp(&project_root, input);
    }

    generate_gresource_xml(&project_root, &blueprint_inputs, &icon_inputs);
}

fn discover_files_with_extension(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(root, &mut files);

    files.retain(|path| path.extension().and_then(OsStr::to_str) == Some(extension));
    files.sort();

    files
}

fn discover_icon_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(root, &mut files);

    files.retain(|path| path.is_file());
    files.sort();

    files
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }

    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {e}", dir.display()));

    let mut paths = Vec::new();
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|e| panic!("Failed to read entry in {}: {e}", dir.display()));
        paths.push(entry.path());
    }

    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_files_recursive(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn output_ui_path(project_root: &Path, input_path: &Path) -> PathBuf {
    let file_name = input_path
        .file_name()
        .unwrap_or_else(|| panic!("Invalid blueprint path: {}", input_path.display()));

    let mut out = project_root.join("resources/ui").join(file_name);
    out.set_extension("ui");
    out
}

fn compile_blp(project_root: &Path, input_path: &Path) {
    println!("cargo:rerun-if-changed={}", input_path.display());

    if input_path.extension().and_then(OsStr::to_str) != Some("blp") {
        panic!(
            "Only .blp files are allowed for blueprint compilation, got: {}",
            input_path.display()
        );
    }

    let output_path = output_ui_path(project_root, input_path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", parent.display()));
    }

    let status = Command::new("blueprint-compiler")
        .arg("compile")
        .arg("--output")
        .arg(&output_path)
        .arg(input_path)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "Failed to execute blueprint-compiler for {}: {e}\n\
                 Make sure `blueprint-compiler` is installed and available in PATH.",
                input_path.display()
            )
        });

    if !status.success() {
        panic!(
            "blueprint-compiler failed for {}\noutput file: {}",
            input_path.display(),
            output_path.display()
        );
    }
}

fn check_duplicate_ui_outputs(project_root: &Path, inputs: &[PathBuf]) {
    let mut seen: HashMap<PathBuf, PathBuf> = HashMap::new();

    for input in inputs {
        let output = output_ui_path(project_root, input);

        if let Some(previous) = seen.insert(output.clone(), input.clone()) {
            panic!(
                "Duplicate UI output path detected:\n  {}\nfrom:\n  {}\n  {}",
                output.display(),
                previous.display(),
                input.display()
            );
        }
    }
}

fn cleanup_generated_ui_files(project_root: &Path, inputs: &[PathBuf]) {
    let ui_dir = project_root.join("resources/ui");

    let expected: BTreeSet<PathBuf> = inputs
        .iter()
        .map(|input| output_ui_path(project_root, input))
        .collect();

    if !ui_dir.exists() {
        fs::create_dir_all(&ui_dir)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", ui_dir.display()));
        return;
    }

    let entries = fs::read_dir(&ui_dir)
        .unwrap_or_else(|e| panic!("Failed to read directory {}: {e}", ui_dir.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|e| panic!("Failed to read entry in {}: {e}", ui_dir.display()));
        let path = entry.path();

        if path.is_file() {
            fs::remove_file(&path)
                .unwrap_or_else(|e| panic!("Failed to remove stale file {}: {e}", path.display()));
        }
    }
}

fn generate_gresource_xml(project_root: &Path, blp_inputs: &[PathBuf], icon_inputs: &[PathBuf]) {
    let output_path = project_root.join("resources/resources.gresource.xml");
    let resources_root = project_root.join("resources");

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!-- Auto-generated by build.rs. Do not edit manually. -->\n");
    xml.push_str("<gresources>\n");

    write_icon_gresources(&mut xml, &resources_root, icon_inputs);
    write_ui_gresource(&mut xml, project_root, &resources_root, blp_inputs);

    xml.push_str("</gresources>\n");

    let old = fs::read_to_string(&output_path).ok();
    if old.as_deref() != Some(xml.as_str()) {
        fs::write(&output_path, xml)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
    }
}

fn write_icon_gresources(xml: &mut String, resources_root: &Path, icon_inputs: &[PathBuf]) {
    let mut groups: BTreeMap<String, Vec<&PathBuf>> = BTreeMap::new();

    for icon in icon_inputs {
        println!("cargo:rerun-if-changed={}", icon.display());

        let rel_to_resources = icon
            .strip_prefix(resources_root)
            .unwrap_or_else(|_| panic!("Icon path is not under resources/: {}", icon.display()));

        let parent = rel_to_resources.parent().unwrap_or_else(|| {
            panic!(
                "Icon path has no parent under resources/: {}",
                rel_to_resources.display()
            )
        });

        let prefix = format!("{APP_RESOURCE_PREFIX}/{}/", to_unix_path(parent));
        groups.entry(prefix).or_default().push(icon);
    }

    for (prefix, files) in groups {
        xml.push_str(&format!(
            "  <gresource prefix=\"{}\">\n",
            xml_escape_attr(&prefix)
        ));

        for icon in files {
            let rel_to_resources = icon.strip_prefix(resources_root).unwrap_or_else(|_| {
                panic!("Icon path is not under resources/: {}", icon.display())
            });

            let alias = rel_to_resources
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_else(|| {
                    panic!("Invalid icon file name: {}", rel_to_resources.display())
                });

            let rel_text = to_unix_path(rel_to_resources);

            if icon.extension().and_then(OsStr::to_str) == Some("svg") {
                xml.push_str(&format!(
                    "    <file preprocess=\"xml-stripblanks\" alias=\"{}\">{}</file>\n",
                    xml_escape_attr(alias),
                    xml_escape_text(&rel_text)
                ));
            } else {
                xml.push_str(&format!(
                    "    <file alias=\"{}\">{}</file>\n",
                    xml_escape_attr(alias),
                    xml_escape_text(&rel_text)
                ));
            }
        }

        xml.push_str("  </gresource>\n");
    }
}

fn write_ui_gresource(
    xml: &mut String,
    project_root: &Path,
    resources_root: &Path,
    blp_inputs: &[PathBuf],
) {
    xml.push_str(&format!(
        "  <gresource prefix=\"{}\">\n",
        APP_RESOURCE_PREFIX
    ));

    for input in blp_inputs {
        let compiled = output_ui_path(project_root, input);
        let rel_to_resources = compiled.strip_prefix(resources_root).unwrap_or_else(|_| {
            panic!(
                "Compiled UI path is not under resources/: {}",
                compiled.display()
            )
        });

        let rel_text = to_unix_path(rel_to_resources);

        xml.push_str(&format!(
            "    <file compressed=\"true\" preprocess=\"xml-stripblanks\">{}</file>\n",
            xml_escape_text(&rel_text)
        ));
    }

    xml.push_str("  </gresource>\n");
}

fn to_unix_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn xml_escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn xml_escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}
