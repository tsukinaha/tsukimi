#[cfg(target_os = "linux")]
pub fn get_etag(path: &std::path::Path) -> Option<String> {
    xattr::get(path, "user.etag")
        .ok()
        .flatten()
        .and_then(|value| String::from_utf8(value).ok())
}

#[cfg(target_os = "linux")]
pub fn set_etag(path: &std::path::Path, etag: &str) -> std::io::Result<()> {
    xattr::set(path, "user.etag", etag.as_bytes())
}

#[cfg(not(target_os = "linux"))]
pub fn get_etag(path: &std::path::Path) -> Option<String> {
    let sidecar_path = etag_sidecar_path(path);
    std::fs::read_to_string(sidecar_path).ok()
}

#[cfg(not(target_os = "linux"))]
pub fn set_etag(path: &std::path::Path, etag: &str) -> std::io::Result<()> {
    let sidecar_path = etag_sidecar_path(path);
    std::fs::write(sidecar_path, etag)
}

#[cfg(not(target_os = "linux"))]
fn etag_sidecar_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut sidecar_path = path.as_os_str().to_owned();
    sidecar_path.push(".etag");
    std::path::PathBuf::from(sidecar_path)
}
