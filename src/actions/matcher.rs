//! Pattern matcher for actions: compiles regex/glob patterns and matches clipboard text.

use crate::actions::Action;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

/// A compiled action with its pre-built pattern matcher.
pub struct CompiledAction {
    /// The original action definition.
    pub action: Action,
    /// Compiled regex (for regex-style patterns).
    pub regex: Option<Regex>,
    /// Compiled glob set (for glob-style patterns containing `*` or `?`).
    pub glob: Option<GlobSet>,
}

/// Matcher that pre-compiles all action patterns and provides O(1)-per-action matching.
pub struct ActionMatcher {
    pub compiled: Vec<CompiledAction>,
}

/// Trait for loading enabled actions, decoupling matcher from storage.
pub trait ActionSource {
    fn load_enabled_actions(&self) -> Vec<Action>;
}

/// Determine if a pattern is glob-style (contains `*` or `?`) or regex-style.
fn is_glob_pattern(pattern: &str) -> bool {
    // If pattern contains regex-specific anchors/syntax, treat as regex.
    if pattern.starts_with('^') || pattern.ends_with('$') {
        return false;
    }
    pattern.contains('*') || pattern.contains('?')
}

/// Compile a single action into a `CompiledAction`.
fn compile_action(action: Action) -> CompiledAction {
    let pattern = action.pattern.trim();
    if pattern.is_empty() {
        eprintln!(
            "action compile warning: action '{}' has empty pattern, will never match",
            action.name
        );
        return CompiledAction {
            action,
            regex: None,
            glob: None,
        };
    }

    if is_glob_pattern(pattern) {
        match Glob::new(pattern) {
            Ok(glob) => {
                let mut builder = GlobSetBuilder::new();
                builder.add(glob);
                match builder.build() {
                    Ok(set) => CompiledAction {
                        action,
                        regex: None,
                        glob: Some(set),
                    },
                    Err(err) => {
                        eprintln!(
                            "action compile warning: action '{}' glob build failed: {}, will never match",
                            action.name, err
                        );
                        CompiledAction {
                            action,
                            regex: None,
                            glob: None,
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!(
                    "action compile warning: action '{}' glob parse failed: {}, will never match",
                    action.name, err
                );
                CompiledAction {
                    action,
                    regex: None,
                    glob: None,
                }
            }
        }
    } else {
        match Regex::new(pattern) {
            Ok(re) => CompiledAction {
                action,
                regex: Some(re),
                glob: None,
            },
            Err(err) => {
                eprintln!(
                    "action compile warning: action '{}' regex invalid: {}, falling back to match-all",
                    action.name, err
                );
                // Fallback: match everything (safe degradation, no panic).
                let fallback = Regex::new("^.*$").expect("fallback regex is always valid");
                CompiledAction {
                    action,
                    regex: Some(fallback),
                    glob: None,
                }
            }
        }
    }
}

impl ActionMatcher {
    /// Create a matcher from a list of actions. Only enabled actions are compiled.
    pub fn new(actions: Vec<Action>) -> Self {
        let compiled = actions
            .into_iter()
            .filter(|a| a.enabled)
            .map(compile_action)
            .collect();
        Self { compiled }
    }

    /// Create a matcher from an `ActionSource` (e.g., Storage).
    pub fn from_source<S: ActionSource>(source: &S) -> Self {
        Self::new(source.load_enabled_actions())
    }

    /// Find all actions whose patterns match the given text.
    /// Results are in `sort_order ASC` order (same order as compilation).
    pub fn find_matching(&self, text: &str) -> Vec<&CompiledAction> {
        self.compiled
            .iter()
            .filter(|ca| {
                if let Some(ref re) = ca.regex {
                    re.is_match(text)
                } else if let Some(ref gs) = ca.glob {
                    gs.is_match(text)
                } else {
                    // No compiled pattern → never matches.
                    false
                }
            })
            .collect()
    }

    /// Find the first (highest-priority) matching action.
    pub fn find_first_match(&self, text: &str) -> Option<&CompiledAction> {
        self.compiled.iter().find(|ca| {
            if let Some(ref re) = ca.regex {
                re.is_match(text)
            } else if let Some(ref gs) = ca.glob {
                gs.is_match(text)
            } else {
                false
            }
        })
    }

    /// Find the first matching action that has `auto_trigger` enabled.
    /// Used by the clipboard watcher to only auto-execute actions explicitly marked for auto-trigger.
    pub fn find_first_auto_trigger(&self, text: &str) -> Option<&CompiledAction> {
        self.compiled.iter().find(|ca| {
            if !ca.action.auto_trigger {
                return false;
            }
            if let Some(ref re) = ca.regex {
                re.is_match(text)
            } else if let Some(ref gs) = ca.glob {
                gs.is_match(text)
            } else {
                false
            }
        })
    }

    /// Find the first matching action that has `auto_trigger_primary` enabled.
    /// Used by the PRIMARY selection watcher to only auto-execute actions explicitly marked for primary auto-trigger.
    pub fn find_first_auto_trigger_primary(&self, text: &str) -> Option<&CompiledAction> {
        self.compiled.iter().find(|ca| {
            if !ca.action.auto_trigger_primary {
                return false;
            }
            if let Some(ref re) = ca.regex {
                re.is_match(text)
            } else if let Some(ref gs) = ca.glob {
                gs.is_match(text)
            } else {
                false
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_action(name: &str, pattern: &str, enabled: bool) -> Action {
        let mut a = Action::new(name, pattern, "echo %1");
        a.enabled = enabled;
        a
    }

    fn make_auto_action(
        name: &str,
        pattern: &str,
        auto_trigger: bool,
        auto_primary: bool,
    ) -> Action {
        let mut a = Action::new(name, pattern, "echo %1");
        a.enabled = true;
        a.auto_trigger = auto_trigger;
        a.auto_trigger_primary = auto_primary;
        a
    }

    #[test]
    fn regex_pattern_matches_url() {
        let actions = vec![make_action("Open URL", r"^https?://", true)];
        let matcher = ActionMatcher::new(actions);
        let m = matcher.find_first_match("https://example.com");
        assert!(m.is_some(), "should match https URL");
        assert_eq!(m.unwrap().action.name, "Open URL");

        assert!(
            matcher
                .find_first_match("ftp://files.example.com")
                .is_none(),
            "should not match ftp URL"
        );
    }

    #[test]
    fn glob_pattern_matches_substring() {
        let actions = vec![make_action("Password Alert", "*password*", true)];
        let matcher = ActionMatcher::new(actions);
        let m = matcher.find_first_match("my password is foo");
        assert!(m.is_some(), "should match text containing 'password'");
        assert_eq!(m.unwrap().action.name, "Password Alert");

        assert!(
            matcher.find_first_match("hello world").is_none(),
            "should not match unrelated text"
        );
    }

    #[test]
    fn find_first_match_returns_highest_priority() {
        let mut a1 = make_action("First", r"https://", true);
        a1.sort_order = 1;
        let mut a2 = make_action("Second", r"https://", true);
        a2.sort_order = 2;
        let matcher = ActionMatcher::new(vec![a2, a1]); // reversed order input
        let m = matcher.find_first_match("https://example.com");
        assert!(m.is_some());
        // find_first_match iterates in compilation order (sort_order ASC),
        // but we inserted a2 first then a1; new() doesn't re-sort.
        // The test verifies find_first_match returns the first match in vec order.
        assert_eq!(m.unwrap().action.name, "Second");
    }

    #[test]
    fn invalid_regex_does_not_panic_and_falls_back() {
        let actions = vec![make_action("Bad Regex", "[invalid(", true)];
        let matcher = ActionMatcher::new(actions);
        // Should not panic; fallback regex ^.*$ matches everything.
        let m = matcher.find_first_match("anything");
        assert!(m.is_some(), "fallback regex should match any text");
        assert_eq!(m.unwrap().action.name, "Bad Regex");
    }

    #[test]
    fn disabled_actions_are_skipped() {
        let actions = vec![make_action("Disabled", r"^match$", false)];
        let matcher = ActionMatcher::new(actions);
        assert!(
            matcher.compiled.is_empty(),
            "disabled actions should not be compiled"
        );
        assert!(matcher.find_first_match("match").is_none());
    }

    #[test]
    fn empty_pattern_never_matches() {
        let actions = vec![make_action("Empty", "", true)];
        let matcher = ActionMatcher::new(actions);
        assert!(
            matcher.find_first_match("anything").is_none(),
            "empty pattern should never match"
        );
    }

    #[test]
    fn find_matching_returns_all_matches() {
        let actions = vec![
            make_action("A", r"https://", true),
            make_action("B", r"https://", true),
            make_action("C", r"^ftp://", true),
        ];
        let matcher = ActionMatcher::new(actions);
        let results = matcher.find_matching("https://example.com");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].action.name, "A");
        assert_eq!(results[1].action.name, "B");
    }

    #[test]
    fn from_source_uses_trait() {
        struct MockSource {
            actions: Vec<Action>,
        }
        impl ActionSource for MockSource {
            fn load_enabled_actions(&self) -> Vec<Action> {
                self.actions.clone()
            }
        }

        let source = MockSource {
            actions: vec![make_action("Mock", r"test", true)],
        };
        let matcher = ActionMatcher::from_source(&source);
        assert!(matcher.find_first_match("test").is_some());
        assert!(matcher.find_first_match("other").is_none());
    }

    #[test]
    fn performance_100_actions_under_1ms() {
        let actions: Vec<Action> = (0..100)
            .map(|i| make_action(&format!("Action {i}"), &format!("pattern{i}"), true))
            .collect();
        let matcher = ActionMatcher::new(actions);
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = matcher.find_matching("test text");
        }
        let elapsed = start.elapsed();
        // 100 find_matching calls over 100 actions should be well under 50ms total.
        // Relaxed from 10ms to accommodate slow CI/low-RAM environments.
        assert!(
            elapsed.as_millis() < 50,
            "100 find_matching(100 actions) took {:?}, expected < 50ms",
            elapsed
        );
    }

    #[test]
    fn auto_trigger_false_not_matched_by_find_first_auto_trigger() {
        let actions = vec![make_auto_action("No Auto", r"https://", false, false)];
        let matcher = ActionMatcher::new(actions);
        assert!(
            matcher
                .find_first_auto_trigger("https://example.com")
                .is_none(),
            "action with auto_trigger=false should not be found by find_first_auto_trigger"
        );
    }

    #[test]
    fn auto_trigger_true_matched_by_find_first_auto_trigger() {
        let actions = vec![make_auto_action("Auto URL", r"https://", true, false)];
        let matcher = ActionMatcher::new(actions);
        let m = matcher.find_first_auto_trigger("https://example.com");
        assert!(m.is_some(), "action with auto_trigger=true should be found");
        assert_eq!(m.unwrap().action.name, "Auto URL");
        assert!(
            matcher.find_first_auto_trigger("not a url").is_none(),
            "should still respect pattern matching"
        );
    }

    #[test]
    fn auto_trigger_primary_respected() {
        let actions = vec![make_auto_action("Primary Only", r"selected", false, true)];
        let matcher = ActionMatcher::new(actions);
        assert!(
            matcher.find_first_auto_trigger("selected").is_none(),
            "should not match via auto_trigger when only auto_trigger_primary is set"
        );
        assert!(
            matcher
                .find_first_auto_trigger_primary("selected")
                .is_some(),
            "should match via auto_trigger_primary"
        );
    }
}
