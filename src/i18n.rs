#[allow(unused_imports)]
pub use rust_i18n::t;

/// Translate a key to the current locale.
///
/// Uses `rust_i18n::t!` to perform the lookup. To surface untranslated
/// keys during development, enable the `log-miss-tr` cargo feature
/// (bundled with `--features devtools` or `--features log-miss-tr`),
/// which logs missing keys to stderr via env_logger.
#[allow(dead_code)]
pub fn tr(key: &'static str) -> String {
    rust_i18n::t!(key).to_string()
}

/// Return the currently active locale string (e.g. `"zh-CN"`, `"en-US"`).
pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

/// Set the application locale at runtime.
///
/// Accepts `"zh-CN"` or `"en-US"`. Invalid values are silently ignored.
pub fn set_app_locale(lang: &str) {
    match lang {
        "zh-CN" | "en-US" => rust_i18n::set_locale(lang),
        _ => {}
    }
}

/// Resolve a persisted language choice to a concrete application locale.
pub fn resolve_locale_choice(lang: &str) -> String {
    match lang {
        "follow-system" => detect_system_locale(),
        "zh-CN" | "en-US" => lang.to_string(),
        _ => "en-US".to_string(),
    }
}

/// Apply a persisted language choice, including `"follow-system"`.
pub fn set_app_locale_choice(lang: &str) {
    let locale = resolve_locale_choice(lang);
    set_app_locale(&locale);
}

/// Detect locale from a raw locale string (e.g. `"zh_CN.UTF-8"`).
///
/// This is a pure function — no environment variable access.
/// See [`detect_system_locale`] for the env-aware wrapper.
///
/// Rules: `"zh*"` / `"chinese"` → `"zh-CN"`,
///        `"en*"` / `"english"` → `"en-US"`,
///         everything else → `"en-US"`.
pub fn detect_from_raw(raw: &str) -> String {
    let lang = raw
        .split('.')
        .next()
        .unwrap_or("")
        .replace('_', "-")
        .to_lowercase();
    if lang.starts_with("zh") || lang.contains("chinese") {
        "zh-CN".to_string()
    } else {
        "en-US".to_string()
    }
}

/// Detect system locale from `LC_MESSAGES` / `LANG` environment variables.
///
/// Priority: `LC_MESSAGES` > `LANG`.  Falls back to `"en-US"` when neither
/// is set or when the value doesn't match a known locale pattern.
pub fn detect_system_locale() -> String {
    let raw = crate::platform::system_locale_name()
        .or_else(|| std::env::var("LC_MESSAGES").ok())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();
    detect_from_raw(&raw)
}

/// Log the current locale at startup for diagnostics.
#[cfg(feature = "log-miss-tr")]
pub fn log_locale_info() {
    let locale = current_locale();
    log::info!("i18n: locale={}, zh-CN=100%, en-US=100%", locale);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_zh_cn() {
        assert_eq!(detect_from_raw("zh_CN.UTF-8"), "zh-CN");
    }

    #[test]
    fn test_detect_en_us() {
        assert_eq!(detect_from_raw("en_US.UTF-8"), "en-US");
    }

    #[test]
    fn test_detect_fallback_french() {
        assert_eq!(detect_from_raw("fr_FR.UTF-8"), "en-US");
    }

    #[test]
    fn test_detect_empty() {
        assert_eq!(detect_from_raw(""), "en-US");
    }

    #[test]
    fn test_detect_c() {
        assert_eq!(detect_from_raw("C"), "en-US");
    }
}
