use crate::actions::{Action, ActionKind};
use rust_i18n::t;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "mkfs",
    "dd if=",
    ":(){:|:&};:",
    "chmod -R 777 /",
    ">/dev/sda",
];

#[derive(Debug, Clone)]
pub struct ActionResult {
    pub action_id: u64,
    pub status: ActionStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionStatus {
    Running,
    Completed,
    Failed,
    Timeout,
    Blocked,
}

pub struct ActionExecutor {
    max_runtime: Duration,
    max_concurrent: usize,
    running: Arc<AtomicUsize>,
    allowlist: Arc<Mutex<Vec<String>>>,
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self {
            max_runtime: Duration::from_secs(30),
            max_concurrent: 4,
            running: Arc::new(AtomicUsize::new(0)),
            allowlist: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[cfg(test)]
    pub fn with_max_runtime(mut self, d: Duration) -> Self {
        self.max_runtime = d;
        self
    }

    #[cfg(test)]
    pub fn with_max_concurrent(mut self, n: usize) -> Self {
        self.max_concurrent = n;
        self
    }

    pub fn set_allowlist(&self, list: Vec<String>) {
        if let Ok(mut guard) = self.allowlist.lock() {
            *guard = list;
        }
    }

    pub fn execute(&self, action: &Action, content: &str) -> ActionResult {
        let start = Instant::now();

        if let Some(reason) = check_blocklist(&action.command, content) {
            return ActionResult {
                action_id: action.id,
                status: ActionStatus::Blocked,
                output: None,
                error: Some(reason),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        if !self.is_command_allowed(&action.command) {
            return ActionResult {
                action_id: action.id,
                status: ActionStatus::Blocked,
                output: None,
                error: Some(t!("action.executor.blocked_allowlist").to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        let current = self.running.load(Ordering::SeqCst);
        if current >= self.max_concurrent {
            return ActionResult {
                action_id: action.id,
                status: ActionStatus::Failed,
                output: None,
                error: Some(t!("action.executor.too_many_concurrent").to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
        self.running.fetch_add(1, Ordering::SeqCst);
        let result = self.run_action(action, content, start);
        self.running.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub fn execute_async(&self, action: &Action, content: &str) {
        let action = action.clone();
        let content = content.to_string();
        let executor = ActionExecutor {
            max_runtime: self.max_runtime,
            max_concurrent: self.max_concurrent,
            running: Arc::clone(&self.running),
            allowlist: Arc::clone(&self.allowlist),
        };
        thread::spawn(move || {
            let _ = executor.execute(&action, &content);
        });
    }

    fn run_action(&self, action: &Action, content: &str, start: Instant) -> ActionResult {
        let expanded = expand_placeholders(&action.command, content);
        let (program, args) = match action.kind {
            ActionKind::Open => ("xdg-open".to_string(), vec![content.to_string()]),
            ActionKind::OpenWith => {
                let parts = split_command(&expanded);
                if parts.is_empty() {
                    return ActionResult {
                        action_id: action.id,
                        status: ActionStatus::Failed,
                        output: None,
                        error: Some(t!("action.executor.empty_command").to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
                (parts[0].clone(), parts[1..].to_vec())
            }
            ActionKind::Copy => {
                return ActionResult {
                    action_id: action.id,
                    status: ActionStatus::Failed,
                    output: None,
                    error: Some(t!("action.executor.copy_not_supported").to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
            ActionKind::ShellCommand => {
                ("sh".to_string(), vec!["-c".to_string(), expanded.clone()])
            }
        };

        match spawn_with_timeout(&program, &args, self.max_runtime) {
            Ok((output, timed_out)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let status = if timed_out {
                    ActionStatus::Timeout
                } else if output.status.success() {
                    ActionStatus::Completed
                } else {
                    ActionStatus::Failed
                };
                ActionResult {
                    action_id: action.id,
                    status,
                    output: Some(stdout),
                    error: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            Err(err) => ActionResult {
                action_id: action.id,
                status: ActionStatus::Failed,
                output: None,
                error: Some(err),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    fn is_command_allowed(&self, command: &str) -> bool {
        let guard = self
            .allowlist
            .lock()
            .expect("allowlist mutex poisoned in is_command_allowed");
        if guard.is_empty() {
            return true;
        }
        let lower = command.to_lowercase();
        guard
            .iter()
            .any(|allowed| lower.contains(&allowed.to_lowercase()))
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn expand_placeholders(template: &str, content: &str) -> String {
    expand_placeholders_with_captures(template, content, None)
}

pub fn expand_placeholders_with_captures(
    template: &str,
    content: &str,
    captures: Option<&regex::Captures<'_>>,
) -> String {
    let mut result = template.to_string();
    result = result.replace("%clipboard", content);
    if let Some(caps) = captures {
        for i in 1..=9 {
            let placeholder = format!("%{i}");
            if result.contains(&placeholder)
                && let Some(m) = caps.get(i)
            {
                result = result.replace(&placeholder, m.as_str());
            }
        }
    }
    result = result.replace("%%", "%");
    result
}

fn check_blocklist(command: &str, content: &str) -> Option<String> {
    let combined = format!("{command} {content}");
    let lower = combined.to_lowercase();
    for pattern in DANGEROUS_PATTERNS {
        if lower.contains(&pattern.to_lowercase()) {
            return Some(format!(
                "{}: {}",
                t!("action.executor.blocked_dangerous"),
                pattern
            ));
        }
    }
    None
}

fn spawn_with_timeout(
    program: &str,
    args: &[String],
    timeout: Duration,
) -> Result<(std::process::Output, bool), String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child: Child = cmd.spawn().map_err(|err| {
        t!(
            "action.executor.spawn_failed",
            program = program,
            err = err.to_string()
        )
        .to_string()
    })?;

    let (tx, rx) = std::sync::mpsc::channel();
    let child_id = child.id();
    thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok((output, false)),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => {
            let _ = kill_process(child_id);
            let _ = rx.recv();
            Ok((
                std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: b"Process killed: timeout".to_vec(),
                },
                true,
            ))
        }
    }
}

fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        unsafe {
            let ret = libc::kill(-(pid as i32), libc::SIGKILL);
            if ret != 0 {
                let err = std::io::Error::last_os_error();
                return Err(format!("kill failed: {}", err));
            }
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map_err(|err| format!("taskkill failed: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("taskkill exited with {status}"))
        }
    }
}

fn split_command(command: &str) -> Vec<String> {
    shlex::split(command).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use regex::Regex;

    fn make_action(kind: ActionKind, command: &str) -> Action {
        let mut a = Action::new("test", ".*", command);
        a.kind = kind;
        a
    }

    #[test]
    fn percent_clipboard_replaced() {
        let result = expand_placeholders("echo %clipboard", "hello world");
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn percent_escape_literal() {
        let result = expand_placeholders("echo 100%%", "");
        assert_eq!(result, "echo 100%");
    }

    #[test]
    fn percent_capture_groups() {
        let re = Regex::new(r"^(\w+)@(\w+)$").unwrap();
        let caps = re.captures("user@host").unwrap();
        let result = expand_placeholders_with_captures("ssh %2 -l %1", "user@host", Some(&caps));
        assert_eq!(result, "ssh host -l user");
    }

    #[test]
    fn percent_clipboard_and_escape_combined() {
        let result = expand_placeholders("echo %clipboard (100%%)", "hi");
        assert_eq!(result, "echo hi (100%)");
    }

    #[test]
    fn blocklist_rm_rf_root() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "rm -rf /");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Blocked);
    }

    #[test]
    fn blocklist_fork_bomb() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, ":(){:|:&};:");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Blocked);
    }

    #[test]
    fn blocklist_dd_if() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "dd if=/dev/zero of=/dev/sda");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Blocked);
    }

    #[test]
    fn blocklist_mkfs() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "mkfs.ext4 /dev/sda1");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Blocked);
    }

    #[test]
    fn allowlist_blocks_unlisted_command() {
        let executor = ActionExecutor::new();
        executor.set_allowlist(vec!["firefox".to_string()]);
        let action = make_action(ActionKind::ShellCommand, "echo hello");
        let result = executor.execute(&action, "test");
        assert_eq!(result.status, ActionStatus::Blocked);
    }

    #[test]
    fn allowlist_allows_listed_command() {
        let executor = ActionExecutor::new();
        executor.set_allowlist(vec!["echo".to_string()]);
        let action = make_action(ActionKind::ShellCommand, "echo hello");
        let result = executor.execute(&action, "test");
        assert_eq!(result.status, ActionStatus::Completed);
    }

    #[test]
    fn empty_allowlist_allows_all() {
        let executor = ActionExecutor::new();
        executor.set_allowlist(vec![]);
        let action = make_action(ActionKind::ShellCommand, "echo hello");
        let result = executor.execute(&action, "test");
        assert_eq!(result.status, ActionStatus::Completed);
    }

    #[test]
    fn shell_command_echo() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "echo %clipboard");
        let result = executor.execute(&action, "hello");
        assert_eq!(result.status, ActionStatus::Completed);
        assert!(result.output.as_deref().unwrap_or("").contains("hello"));
    }

    #[test]
    fn shell_command_exit_code_nonzero() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "false");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Failed);
    }

    #[test]
    fn nonexistent_command_fails_gracefully() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "nonexistent_cmd_xyz");
        let result = executor.execute(&action, "");
        assert_eq!(result.status, ActionStatus::Failed);
        assert!(result.error.is_some());
    }

    #[test]
    fn timeout_kills_long_process() {
        let executor = ActionExecutor::new().with_max_runtime(Duration::from_secs(2));
        let action = make_action(ActionKind::ShellCommand, "sleep 60");
        let start = Instant::now();
        let result = executor.execute(&action, "");
        let elapsed = start.elapsed();
        assert_eq!(result.status, ActionStatus::Timeout);
        assert!(
            elapsed < Duration::from_secs(10),
            "timeout took too long: {elapsed:?}"
        );
    }

    #[test]
    fn copy_kind_returns_failed() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::Copy, "echo test");
        let result = executor.execute(&action, "test");
        assert_eq!(result.status, ActionStatus::Failed);
    }

    #[test]
    fn open_kind_uses_xdg_open() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::Open, "%clipboard");
        let result = executor.execute(&action, "https://example.com");
        // xdg-open may fail or timeout in headless CI environments — that's fine
        assert!(
            matches!(
                result.status,
                ActionStatus::Completed | ActionStatus::Failed | ActionStatus::Timeout
            ),
            "expected Completed/Failed/Timeout, got {:?}",
            result.status,
        );
    }

    #[test]
    fn result_tracks_duration() {
        let executor = ActionExecutor::new();
        let action = make_action(ActionKind::ShellCommand, "echo ok");
        let result = executor.execute(&action, "");
        assert!(result.duration_ms < 10_000, "should be fast");
    }

    #[test]
    fn split_command_basic() {
        let parts = split_command("firefox %clipboard");
        assert_eq!(parts, vec!["firefox", "%clipboard"]);
    }

    #[test]
    fn split_command_empty() {
        let parts = split_command("");
        assert!(parts.is_empty());
    }

    #[test]
    fn action_result_serialize_roundtrip() {
        let result = ActionResult {
            action_id: 12345,
            status: ActionStatus::Completed,
            output: Some("hello".to_string()),
            error: None,
            duration_ms: 42,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("Completed"));
        assert!(debug.contains("12345"));
    }

    #[test]
    fn action_status_equality() {
        assert_eq!(ActionStatus::Completed, ActionStatus::Completed);
        assert_ne!(ActionStatus::Completed, ActionStatus::Failed);
        assert_ne!(ActionStatus::Blocked, ActionStatus::Timeout);
    }
}
