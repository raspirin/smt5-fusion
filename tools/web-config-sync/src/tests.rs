use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::*;

const HTML: &str = include_str!("../../../crates/smt5-fusion-web/index.html");
const PRODUCTION: &str = "production\0smt5-fusion-web:theme:v1\0local";
const PREVIEW: &str = "preview\0smt5-fusion-web:preview:theme:v1\0test-build";

fn length(mut value: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value >= 128 {
        bytes.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    bytes.push(value as u8);
    bytes
}

fn section(name: &[u8], content: &[u8]) -> Vec<u8> {
    let mut payload = length(name.len());
    payload.extend(name);
    payload.extend(content);
    let mut bytes = vec![0];
    bytes.extend(length(payload.len()));
    bytes.extend(payload);
    bytes
}

fn wasm(config: &str) -> Vec<u8> {
    let mut bytes = b"\0asm\x01\0\0\0".to_vec();
    bytes.extend(section(b"unrelated", &[0; 200]));
    bytes.extend(section(b"smt5-web-config", config.as_bytes()));
    bytes
}

#[test]
fn reads_only_the_named_custom_section() {
    for config in [PRODUCTION, PREVIEW] {
        assert_eq!(read_config(&wasm(config)).unwrap(), config);
    }
    let mut bytes = b"\0asm\x01\0\0\0".to_vec();
    bytes.extend(section(b"other", PREVIEW.as_bytes()));
    assert!(read_config(&bytes).is_err());
}

#[test]
fn malformed_and_duplicate_sections_fail() {
    let valid = wasm(PREVIEW);
    for end in 0..valid.len() {
        assert!(
            read_config(&valid[..end]).is_err(),
            "Accepted truncated byte {end}"
        );
    }
    let mut duplicate = valid.clone();
    duplicate.extend(section(b"smt5-web-config", PRODUCTION.as_bytes()));
    assert!(read_config(&duplicate).is_err());
    let mut invalid = valid;
    invalid[0] = 1;
    assert!(read_config(&invalid).is_err());
    let mut invalid = b"\0asm\x01\0\0\0".to_vec();
    invalid.extend(section(b"smt5-web-config", &[255]));
    assert!(read_config(&invalid).is_err());
    for bytes in [
        &[128_u8][..],
        &[255, 255, 255, 255, 16],
        &[128, 128, 128, 128, 128, 0],
    ] {
        assert!(read_size(&mut &bytes[..]).is_err());
    }
}

#[test]
fn html_uses_the_compiled_config_without_changing_assets() {
    assert_eq!(patch_html(HTML, PRODUCTION).unwrap(), HTML);
    let preview = patch_html(HTML, PREVIEW).unwrap();
    assert!(preview.contains("name=\"smt5-build-channel\" content=\"preview\""));
    assert!(preview.contains("name=\"smt5-build-id\" content=\"test-build\""));
    assert!(preview.contains("getItem(\"smt5-fusion-web:preview:theme:v1\")"));
    assert!(!preview.contains("smt5-fusion-web:theme:v1"));
    assert_eq!(
        preview
            .split("<title>")
            .nth(1)
            .unwrap()
            .split("</title>")
            .next(),
        HTML.split("<title>")
            .nth(1)
            .unwrap()
            .split("</title>")
            .next()
    );
    assert_eq!(preview.split("<base ").nth(1), HTML.split("<base ").nth(1));
    for forbidden in ["location", "pathname", "hostname", "WebSocket"] {
        assert!(!preview.contains(forbidden));
    }
}

#[test]
fn minified_meta_tags_and_html_escaping_are_supported() {
    let minified = HTML
        .replace(
            "name=\"smt5-build-channel\" content=\"production\" />",
            "name=smt5-build-channel content=production>",
        )
        .replace(
            "name=\"smt5-build-id\" content=\"local\" />",
            "name=smt5-build-id content=local>",
        );
    let output = patch_html(
        &minified,
        "preview\0smt5-fusion-web:preview:theme:v1\0<&\"build>",
    )
    .unwrap();
    assert!(output.contains("content=\"&lt;&amp;&quot;build&gt;\""));
    assert!(output.contains("content=\"preview\""));
}

#[test]
fn missing_template_fields_and_unsafe_configuration_fail() {
    for value in [
        "",
        "preview",
        "preview\0key",
        "preview\0key\0id\0extra",
        "unknown\0key\0id",
        "preview\0\0id",
        "preview\0key\0",
        "preview\0key\"\0id",
    ] {
        assert!(patch_html(HTML, value).is_err(), "Accepted {value:?}");
    }
    for marker in [
        "smt5-build-channel",
        "smt5-build-id",
        "smt5-fusion-web:theme:v1",
    ] {
        assert!(patch_html(&HTML.replace(marker, "removed"), PREVIEW).is_err());
        assert!(patch_html(&HTML.replace(marker, &format!("{marker}{marker}")), PREVIEW).is_err());
    }
}

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = env::temp_dir().join(format!(
            "smt5-web-config-sync-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn the_hook_modifies_only_html_and_accepts_hashed_or_unhashed_ui_names() {
    for name in [
        "smt5-fusion-web_bg.wasm",
        "smt5-fusion-web-0123456789abcdef_bg.wasm",
    ] {
        let directory = Directory::new();
        fs::write(directory.0.join("index.html"), HTML).unwrap();
        let bytes = wasm(PREVIEW);
        fs::write(directory.0.join(name), &bytes).unwrap();
        fs::write(directory.0.join("smt5-fusion-worker_bg.wasm"), b"untouched").unwrap();
        process_html(&directory.0).unwrap();
        assert_eq!(
            fs::read_to_string(directory.0.join("index.html")).unwrap(),
            patch_html(HTML, PREVIEW).unwrap()
        );
        assert_eq!(fs::read(directory.0.join(name)).unwrap(), bytes);
        assert_eq!(
            fs::read(directory.0.join("smt5-fusion-worker_bg.wasm")).unwrap(),
            b"untouched"
        );
    }
}

#[test]
fn missing_ambiguous_or_invalid_ui_assets_leave_html_unchanged() {
    let directory = Directory::new();
    let html = directory.0.join("index.html");
    fs::write(&html, HTML).unwrap();
    assert!(process_html(&directory.0).is_err());
    let ui = directory.0.join("smt5-fusion-web_bg.wasm");
    fs::write(&ui, b"invalid").unwrap();
    assert!(process_html(&directory.0).is_err());
    fs::write(&ui, wasm(PREVIEW)).unwrap();
    fs::write(
        directory.0.join("smt5-fusion-web-0123456789abcdef_bg.wasm"),
        wasm(PRODUCTION),
    )
    .unwrap();
    assert!(process_html(&directory.0).is_err());
    assert_eq!(fs::read_to_string(&html).unwrap(), HTML);
}
