//! WM_CLASS glob matching engine for application exclusion.
//!
//! Compiles user-provided glob patterns (e.g. `keepassxc`, `*password*`) into
//! a [`GlobSet`] at startup so that per-capture matching is O(1).  Patterns
//! are matched against the `WM_CLASS` class name reported by X11 for the
//! focused window.

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

/// Operating mode for the window exclusion/allowlist system.
///
/// - `Blacklist` (default): exclude matching windows from capture.
/// - `Whitelist`: only capture from matching windows (v1.1 deferred stub).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExclusionMode {
    #[default]
    Blacklist,
    Whitelist,
}

/// Compiled blacklist of WM_CLASS patterns.
///
/// Constructed once from the user's `app_exclusion_list` preference and
/// reused for every clipboard capture cycle.
pub struct AppBlacklist {
    #[allow(dead_code)]
    patterns: Vec<String>,
    compiled: GlobSet,
}

impl AppBlacklist {
    /// Compile a set of glob patterns for O(1) matching.
    ///
    /// When `patterns` is empty the blacklist is a no-op: [`is_match`](Self::is_match)
    /// always returns `false` without touching the glob engine.
    pub fn new(patterns: Vec<String>) -> Self {
        if patterns.is_empty() {
            return Self {
                patterns,
                compiled: GlobSet::empty(),
            };
        }
        let mut builder = GlobSetBuilder::new();
        for pat in &patterns {
            if let Ok(glob) = Glob::new(pat) {
                builder.add(glob);
            }
        }
        let compiled = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self { patterns, compiled }
    }

    /// Convenience factory — identical to [`new`](Self::new).
    pub fn from_vec(patterns: Vec<String>) -> Self {
        Self::new(patterns)
    }

    /// Returns `true` when `wm_class` matches any compiled pattern.
    ///
    /// Empty blacklist (no patterns) always returns `false`.
    pub fn is_match(&self, wm_class: &str) -> bool {
        if wm_class.is_empty() {
            return false;
        }
        self.compiled.is_match(wm_class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_pattern_matches_wm_class() {
        let bl = AppBlacklist::new(vec!["*keepassxc*".to_string()]);
        assert!(bl.is_match("org.keepassxc.KeePassXC"));
        assert!(!bl.is_match("org.kde.dolphin"));
    }

    #[test]
    fn wildcard_pattern_matches_glob() {
        let bl = AppBlacklist::new(vec!["*password*".to_string()]);
        assert!(bl.is_match("org.keepassxc.KeePassXC"));
        assert!(bl.is_match("my-password-manager"));
        assert!(!bl.is_match("org.kde.dolphin"));
    }

    #[test]
    fn exact_pattern_case_sensitive() {
        let bl = AppBlacklist::new(vec!["firefox".to_string()]);
        assert!(bl.is_match("firefox"));
        assert!(!bl.is_match("Firefox"));
    }

    #[test]
    fn empty_patterns_never_match() {
        let bl = AppBlacklist::new(vec![]);
        assert!(!bl.is_match("org.keepassxc.KeePassXC"));
        assert!(!bl.is_match("anything"));
        assert!(!bl.is_match(""));
    }

    #[test]
    fn empty_wm_class_never_matches() {
        let bl = AppBlacklist::new(vec!["*".to_string()]);
        assert!(!bl.is_match(""));
    }

    #[test]
    fn multiple_patterns_any_match() {
        let bl = AppBlacklist::new(vec!["*keepassxc*".to_string(), "*bitwarden*".to_string()]);
        assert!(bl.is_match("org.keepassxc.KeePassXC"));
        assert!(bl.is_match("com.bitwarden.desktop"));
        assert!(!bl.is_match("org.kde.dolphin"));
    }

    #[test]
    fn compile_100_patterns_under_10ms() {
        let patterns: Vec<String> = (0..100).map(|i| format!("pattern_{i}_*")).collect();
        let start = std::time::Instant::now();
        let bl = AppBlacklist::new(patterns);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 10,
            "compiling 100 patterns took {}ms (limit: 10ms)",
            elapsed.as_millis()
        );
        assert!(bl.is_match("pattern_50_suffix"));
    }

    #[test]
    fn from_vec_behaves_like_new() {
        let bl = AppBlacklist::from_vec(vec!["*test*".to_string()]);
        assert!(bl.is_match("my-test-app"));
        assert!(!bl.is_match("unrelated"));
    }
}
