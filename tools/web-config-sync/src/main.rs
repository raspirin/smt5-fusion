use std::{env, error::Error, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let directory = env::var_os("TRUNK_STAGING_DIR")
        .ok_or("web-config-sync must be run as a Trunk post_build hook")?;
    process_html(Path::new(&directory))
}

fn process_html(directory: &Path) -> Result<()> {
    let mut ui = None;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_ui = name == "smt5-fusion-web_bg.wasm"
            || (name.starts_with("smt5-fusion-web-") && name.ends_with("_bg.wasm"));
        if is_ui && ui.replace(entry.path()).is_some() {
            return Err("Expected exactly one UI WASM file".into());
        }
    }
    let wasm = fs::read(ui.ok_or("Missing UI WASM file")?)?;
    let config = read_config(&wasm)?;
    let path = directory.join("index.html");
    let html = patch_html(&fs::read_to_string(&path)?, config)?;
    fs::write(path, html)?;
    Ok(())
}

fn take<'a>(bytes: &mut &'a [u8], size: usize) -> Result<&'a [u8]> {
    let (value, rest) = bytes.split_at_checked(size).ok_or("Truncated WASM data")?;
    *bytes = rest;
    Ok(value)
}

fn read_size(bytes: &mut &[u8]) -> Result<usize> {
    let mut size = 0_u32;
    for shift in (0..35).step_by(7) {
        let byte = take(bytes, 1)?[0];
        if shift == 28 && byte > 15 {
            return Err("Invalid WASM section length".into());
        }
        size |= u32::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(size as usize);
        }
    }
    Err("Invalid WASM section length".into())
}

fn read_config(mut wasm: &[u8]) -> Result<&str> {
    if take(&mut wasm, 8)? != b"\0asm\x01\0\0\0" {
        return Err("Invalid WASM header".into());
    }
    let mut config = None;
    while !wasm.is_empty() {
        let id = take(&mut wasm, 1)?[0];
        let size = read_size(&mut wasm)?;
        let mut section = take(&mut wasm, size)?;
        if id == 0 {
            let size = read_size(&mut section)?;
            if take(&mut section, size)? == b"smt5-web-config" {
                if config.is_some() {
                    return Err("Duplicate smt5-web-config section".into());
                }
                config = Some(std::str::from_utf8(section)?);
            }
        }
    }
    config.ok_or_else(|| "Missing smt5-web-config section in UI WASM".into())
}

fn replace_once(html: &str, before: &str, after: &str) -> Result<String> {
    if html.matches(before).count() != 1 {
        return Err(format!("Expected exactly one {before} in startup HTML").into());
    }
    Ok(html.replacen(before, after, 1))
}

fn set_meta(html: &str, name: &str, value: &str) -> Result<String> {
    if html.matches(name).count() != 1 {
        return Err(format!("Expected exactly one {name} meta tag").into());
    }
    let position = html.find(name).ok_or("Missing meta tag name")?;
    let start = html[..position].rfind("<meta ").ok_or("Missing meta tag")?;
    let end = position + html[position..].find('>').ok_or("Unclosed meta tag")? + 1;
    if html[start + 1..end - 1].contains(['<', '>']) {
        return Err("Invalid meta tag".into());
    }
    let value = value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut patched = html.to_owned();
    patched.replace_range(
        start..end,
        &format!("<meta name=\"{name}\" content=\"{value}\" />"),
    );
    Ok(patched)
}

fn patch_html(html: &str, config: &str) -> Result<String> {
    let parts = config.split('\0').collect::<Vec<_>>();
    let [channel, theme_key, build_id] = parts.as_slice() else {
        return Err("Invalid smt5-web-config fields".into());
    };
    if !matches!(*channel, "production" | "preview")
        || theme_key.is_empty()
        || !theme_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":-".contains(&byte))
        || build_id.is_empty()
    {
        return Err("Invalid smt5-web-config values".into());
    }
    let html = replace_once(html, "smt5-fusion-web:theme:v1", theme_key)?;
    let html = set_meta(&html, "smt5-build-channel", channel)?;
    set_meta(&html, "smt5-build-id", build_id)
}

#[cfg(test)]
mod tests;
