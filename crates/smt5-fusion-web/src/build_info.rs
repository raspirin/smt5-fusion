#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const IS_PREVIEW: bool = cfg!(feature = "preview");

#[cfg(any(target_arch = "wasm32", test))]
const BUILD_ID: &str = match option_env!("SMT5_BUILD_ID") {
    Some(id) => id,
    None => "local",
};

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const STORAGE_KEYS: StorageKeys = StorageKeys::for_preview(IS_PREVIEW);

#[cfg(any(target_arch = "wasm32", test))]
const CHANNEL: &str = if IS_PREVIEW { "preview" } else { "production" };

#[cfg(any(target_arch = "wasm32", test))]
const CONFIG_SIZE: usize = CHANNEL.len() + STORAGE_KEYS.theme.len() + BUILD_ID.len() + 2;

// Trunk uses this UI-only section to initialize the theme before WASM starts.
#[cfg(any(target_arch = "wasm32", test))]
#[cfg_attr(target_arch = "wasm32", used)]
#[cfg_attr(target_arch = "wasm32", unsafe(link_section = "smt5-web-config"))]
static BOOTSTRAP_CONFIG: [u8; CONFIG_SIZE] = {
    let parts = [CHANNEL, STORAGE_KEYS.theme, BUILD_ID];
    let mut config = [0; CONFIG_SIZE];
    let mut offset = 0;
    let mut part = 0;
    while part < parts.len() {
        if part > 0 {
            offset += 1;
        }
        let bytes = parts[part].as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            config[offset] = bytes[index];
            offset += 1;
            index += 1;
        }
        part += 1;
    }
    config
};

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StorageKeys {
    pub(crate) locale: &'static str,
    pub(crate) theme: &'static str,
    pub(crate) form: &'static str,
}

#[cfg(any(target_arch = "wasm32", test))]
impl StorageKeys {
    const fn for_preview(preview: bool) -> Self {
        if preview {
            Self {
                locale: "smt5-fusion-web:preview:locale:v1",
                theme: "smt5-fusion-web:preview:theme:v1",
                form: "smt5-fusion-web:preview:search-form:v1",
            }
        } else {
            Self {
                locale: "smt5-fusion-web:locale:v1",
                theme: "smt5-fusion-web:theme:v1",
                form: "smt5-fusion-web:search-form:v1",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn production_storage_keys_remain_compatible_and_preview_keys_are_disjoint() {
        let production = StorageKeys::for_preview(false);
        let preview = StorageKeys::for_preview(true);
        assert_eq!(production.locale, "smt5-fusion-web:locale:v1");
        assert_eq!(production.theme, "smt5-fusion-web:theme:v1");
        assert_eq!(production.form, "smt5-fusion-web:search-form:v1");
        let keys = BTreeSet::from([
            production.locale,
            production.theme,
            production.form,
            preview.locale,
            preview.theme,
            preview.form,
        ]);
        assert_eq!(keys.len(), 6);
        for key in [preview.locale, preview.theme, preview.form] {
            assert!(key.starts_with("smt5-fusion-web:preview:"));
        }
    }

    #[test]
    fn bootstrap_metadata_uses_the_compiled_channel_and_storage_key() {
        let fields = std::str::from_utf8(&BOOTSTRAP_CONFIG)
            .unwrap()
            .split('\0')
            .collect::<Vec<_>>();
        assert_eq!(fields, [CHANNEL, STORAGE_KEYS.theme, BUILD_ID]);
        assert_eq!(CHANNEL, if IS_PREVIEW { "preview" } else { "production" });
    }

    #[test]
    fn the_feature_selects_all_storage_keys_together() {
        assert_eq!(IS_PREVIEW, cfg!(feature = "preview"));
        assert_eq!(STORAGE_KEYS, StorageKeys::for_preview(IS_PREVIEW));
    }
}
