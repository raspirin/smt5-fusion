use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::*;

const HTML: &str = include_str!("../../../crates/smt5-fusion-web/index.html");
const PRODUCTION: &str = "production\0smt5-fusion-web:theme:v1\0local";
const PREVIEW: &str = "preview\0smt5-fusion-web:preview:theme:v1\0test-build";
const WORKER_BYTES: [&[u8]; 3] = [
    b"export default async function init() { return fetch(new URL('smt5-fusion-worker_bg.wasm', import.meta.url)); }\n",
    b"\0asm\x01\0\0\0",
    b"import init from './smt5-fusion-worker.js';await init();",
];
const WORKER_URL: &str = "./worker/8859d4adf1884c51/smt5-fusion-worker_loader.js";

fn write_worker(directory: &Path) {
    for (name, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES) {
        fs::write(directory.join(name), bytes).unwrap();
    }
}

fn assert_worker_unmoved(directory: &Path) {
    for (name, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES) {
        assert_eq!(fs::read(directory.join(name)).unwrap(), bytes);
    }
}

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
fn the_hook_versions_worker_assets_and_preserves_all_asset_bytes() {
    for name in [
        "smt5-fusion-web_bg.wasm",
        "smt5-fusion-web-0123456789abcdef_bg.wasm",
    ] {
        for config in [PRODUCTION, PREVIEW] {
            let directory = Directory::new();
            fs::write(directory.0.join("index.html"), HTML).unwrap();
            let bytes = wasm(config);
            fs::write(directory.0.join(name), &bytes).unwrap();
            fs::write(directory.0.join("style.css"), b"body {}").unwrap();
            write_worker(&directory.0);
            process(&directory.0).unwrap();
            assert_eq!(
                fs::read_to_string(directory.0.join("index.html")).unwrap(),
                set_meta(
                    &patch_html(HTML, config).unwrap(),
                    "smt5-worker-url",
                    WORKER_URL,
                )
                .unwrap()
            );
            assert_eq!(fs::read(directory.0.join(name)).unwrap(), bytes);
            assert_eq!(fs::read(directory.0.join("style.css")).unwrap(), b"body {}");
            let entry = directory.0.join(WORKER_URL.strip_prefix("./").unwrap());
            for (name, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES) {
                assert!(!directory.0.join(name).exists());
                assert_eq!(fs::read(entry.parent().unwrap().join(name)).unwrap(), bytes);
            }
            assert_eq!(fs::read_dir(directory.0.join("worker")).unwrap().count(), 1);
            assert_eq!(fs::read_dir(entry.parent().unwrap()).unwrap().count(), 3);
        }
    }
}

#[test]
fn missing_ambiguous_or_invalid_ui_assets_leave_staging_unchanged() {
    let directory = Directory::new();
    let html = directory.0.join("index.html");
    fs::write(&html, HTML).unwrap();
    write_worker(&directory.0);
    assert!(process(&directory.0).is_err());
    let ui = directory.0.join("smt5-fusion-web_bg.wasm");
    fs::write(&ui, b"invalid").unwrap();
    assert!(process(&directory.0).is_err());
    fs::write(&ui, wasm(PREVIEW)).unwrap();
    fs::write(
        directory.0.join("smt5-fusion-web-0123456789abcdef_bg.wasm"),
        wasm(PRODUCTION),
    )
    .unwrap();
    assert!(process(&directory.0).is_err());
    assert_eq!(fs::read_to_string(&html).unwrap(), HTML);
    assert_worker_unmoved(&directory.0);
    assert!(!directory.0.join("worker").exists());
}

#[test]
fn worker_fingerprint_is_independent_of_ui_and_file_creation_order() {
    for config in [PRODUCTION, PREVIEW] {
        let directory = Directory::new();
        fs::write(directory.0.join("index.html"), config).unwrap();
        fs::write(directory.0.join("smt5-fusion-web_bg.wasm"), wasm(config)).unwrap();
        fs::write(directory.0.join("favicon.svg"), config).unwrap();
        for (name, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES).rev() {
            fs::write(directory.0.join(name), bytes).unwrap();
        }
        for _ in 0..2 {
            assert_eq!(
                WorkerAssets::read(&directory.0).unwrap().entry_url(),
                WORKER_URL
            );
        }
        assert_worker_unmoved(&directory.0);
    }
}

#[test]
fn changing_any_worker_file_changes_the_fingerprint() {
    for (name, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES) {
        let directory = Directory::new();
        write_worker(&directory.0);
        let mut changed = bytes.to_vec();
        changed[0] ^= 1;
        fs::write(directory.0.join(name), changed).unwrap();
        assert_ne!(
            WorkerAssets::read(&directory.0).unwrap().entry_url(),
            WORKER_URL,
            "Ignored changes to {name}"
        );
    }
}

#[test]
fn worker_fingerprint_encodes_file_boundaries() {
    let directory = Directory::new();
    write_worker(&directory.0);
    fs::write(directory.0.join(worker::FILES[0]), b"ab").unwrap();
    fs::write(directory.0.join(worker::FILES[1]), b"c").unwrap();
    let first = WorkerAssets::read(&directory.0).unwrap().entry_url();
    fs::write(directory.0.join(worker::FILES[0]), b"a").unwrap();
    fs::write(directory.0.join(worker::FILES[1]), b"bc").unwrap();
    assert_ne!(WorkerAssets::read(&directory.0).unwrap().entry_url(), first);
}

#[test]
fn missing_or_empty_worker_assets_leave_staging_unchanged() {
    for name in worker::FILES {
        for missing in [true, false] {
            let directory = Directory::new();
            let html = directory.0.join("index.html");
            fs::write(&html, HTML).unwrap();
            fs::write(
                directory.0.join("smt5-fusion-web_bg.wasm"),
                wasm(PRODUCTION),
            )
            .unwrap();
            write_worker(&directory.0);
            if missing {
                fs::remove_file(directory.0.join(name)).unwrap();
            } else {
                fs::write(directory.0.join(name), b"").unwrap();
            }
            assert!(
                process(&directory.0)
                    .unwrap_err()
                    .to_string()
                    .contains(name)
            );
            assert_eq!(fs::read_to_string(&html).unwrap(), HTML);
            assert!(!directory.0.join("worker").exists());
            for (other, bytes) in worker::FILES.into_iter().zip(WORKER_BYTES) {
                if other != name {
                    assert_eq!(fs::read(directory.0.join(other)).unwrap(), bytes);
                }
            }
        }
    }
}

#[test]
fn invalid_worker_metadata_leaves_staging_unchanged() {
    for source in [
        HTML.replace("smt5-worker-url", "removed"),
        HTML.replace("smt5-worker-url", "smt5-worker-url smt5-worker-url"),
    ] {
        let directory = Directory::new();
        let html = directory.0.join("index.html");
        fs::write(&html, &source).unwrap();
        fs::write(
            directory.0.join("smt5-fusion-web_bg.wasm"),
            wasm(PRODUCTION),
        )
        .unwrap();
        write_worker(&directory.0);
        assert!(process(&directory.0).is_err());
        assert_eq!(fs::read_to_string(&html).unwrap(), source);
        assert_worker_unmoved(&directory.0);
        assert!(!directory.0.join("worker").exists());
    }
}

#[test]
fn minified_worker_metadata_is_supported() {
    let directory = Directory::new();
    let html = directory.0.join("index.html");
    fs::write(
        &html,
        HTML.replace(
            "<meta name=\"smt5-worker-url\" content=\"\" />",
            "<meta name=smt5-worker-url content=\"\">",
        ),
    )
    .unwrap();
    fs::write(
        directory.0.join("smt5-fusion-web_bg.wasm"),
        wasm(PRODUCTION),
    )
    .unwrap();
    write_worker(&directory.0);
    process(&directory.0).unwrap();
    assert_eq!(
        fs::read_to_string(&html).unwrap(),
        set_meta(HTML, "smt5-worker-url", WORKER_URL).unwrap()
    );
}

#[test]
fn existing_worker_destination_is_not_overwritten() {
    let directory = Directory::new();
    let html = directory.0.join("index.html");
    fs::write(&html, HTML).unwrap();
    fs::write(
        directory.0.join("smt5-fusion-web_bg.wasm"),
        wasm(PRODUCTION),
    )
    .unwrap();
    write_worker(&directory.0);
    let entry = directory.0.join(WORKER_URL.strip_prefix("./").unwrap());
    fs::create_dir_all(entry.parent().unwrap()).unwrap();
    fs::write(&entry, b"existing asset").unwrap();
    assert!(process(&directory.0).is_err());
    assert_eq!(fs::read_to_string(&html).unwrap(), HTML);
    assert_eq!(fs::read(entry).unwrap(), b"existing asset");
    assert_worker_unmoved(&directory.0);
}
