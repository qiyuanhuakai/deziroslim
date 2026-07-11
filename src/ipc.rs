//! Inter-process communication for CLI ↔ GUI coordination.
//!
//! Provides a TCP socket server (on 127.0.0.1) so that the `dzc-slim` binary
//! can send commands to a running GUI instance. Protocol is JSON Lines: one
//! JSON object per line, terminated by `\n`.
//!
//! - **Request**: `{"cmd":"list","args":{...}}`
//! - **Success**: `{"ok":true,"data":...}`
//! - **Error**:   `{"ok":false,"error":{"code":N,"message":"..."}}`
//!
//! The server binds to an OS-assigned port on 127.0.0.1 and writes the port
//! number plus a per-process random token to a file (`port_file`). The client
//! reads that file to discover and authenticate the local session. This works
//! cross-platform (Linux, Windows, macOS).

use crate::model::{ClipboardEntry, ClipboardKind};
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

// ── Error type ────────────────────────────────────────────────────────

/// Errors returned by the IPC layer. Each variant maps to a distinct CLI
/// exit code so that `dzc-slim` can translate them into user-visible
/// diagnostics.
///
/// | Variant            | Exit code |
/// |--------------------|-----------|
/// | `ConnectionRefused`| 2         |
/// | `InvalidJson`      | 3         |
/// | `UnknownCommand`   | 4         |
/// | `NotFound`         | 5         |
/// | `IpcDisabled`      | 6         |
/// | other              | 1         |
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("connection refused – is deziroslim running?")]
    ConnectionRefused,
    #[error("IPC timed out")]
    Timeout,
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("unknown command (code {0})")]
    UnknownCommand(i32),
    #[error("entry not found")]
    NotFound,
    #[error("IPC is disabled")]
    IpcDisabled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Storage(String),
}

impl IpcError {
    /// Map error variant to a CLI exit code (see struct-level table).
    pub fn exit_code(&self) -> i32 {
        match self {
            IpcError::ConnectionRefused => 2,
            IpcError::InvalidJson(_) => 3,
            IpcError::UnknownCommand(code) => *code,
            IpcError::NotFound => 5,
            IpcError::IpcDisabled => 6,
            _ => 1,
        }
    }
}

// ── JSON Lines protocol types ─────────────────────────────────────────

/// A single request line sent by the CLI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub cmd: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// A single response line sent back by the server.
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcErrorBody>,
}

/// Structured error payload embedded in a failed [`IpcResponse`].
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcErrorBody {
    pub code: i32,
    pub message: String,
}

impl IpcResponse {
    /// Build a success response wrapping arbitrary JSON data.
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build an error response from an [`IpcError`].
    pub fn err(e: &IpcError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(IpcErrorBody {
                code: e.exit_code(),
                message: e.to_string(),
            }),
        }
    }
}

// ── Server ────────────────────────────────────────────────────────────

/// IPC server that listens on a TCP socket (127.0.0.1) and dispatches
/// clipboard commands to the shared [`Storage`].
///
/// The server writes the bound port number and random token to a file so
/// that clients can discover and authenticate the local session.
pub struct IpcServer {
    pub port_file: PathBuf,
}

impl IpcServer {
    /// Resolve the default port-file path:
    /// 1. `$XDG_RUNTIME_DIR/deziroslim.port` (Linux, when set)
    /// 2. `{data_local_dir}/deziroslim/port` (cross-platform fallback)
    pub fn port_file_default() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR")
                && !runtime.is_empty()
            {
                return PathBuf::from(runtime).join("deziroslim.port");
            }
        }
        let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
        base.join("deziroslim").join("port")
    }

    /// Start the IPC server in a background thread.
    ///
    /// - Removes a stale port file if one already exists.
    /// - Binds to 127.0.0.1 on an OS-assigned port.
    /// - Writes the port number and random token to `port_file`.
    /// - Spawns a daemon thread that accepts connections in a loop.
    ///
    /// Returns the server handle so the caller can inspect the port file path.
    pub fn start(storage: Arc<Storage>, port_file: PathBuf) -> Result<Self, IpcError> {
        // Clean up stale port file from a previous crashed instance.
        if port_file.exists() {
            std::fs::remove_file(&port_file)?;
        }

        // Ensure parent directory exists.
        if let Some(parent) = port_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Bind to OS-assigned port on localhost.
        let listener = TcpListener::bind("127.0.0.1:0").map_err(IpcError::Io)?;
        let port = listener.local_addr().map_err(IpcError::Io)?.port();

        let token = new_session_token();
        std::fs::write(&port_file, format!("{port}\n{token}\n")).map_err(IpcError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&port_file, perms)?;
        }

        thread::Builder::new()
            .name("ipc-server".into())
            .spawn(move || {
                Self::accept_loop(listener, storage, token);
            })
            .map_err(IpcError::Io)?;

        Ok(Self { port_file })
    }

    /// Accept loop – runs until the listener is dropped or the process exits.
    fn accept_loop(listener: TcpListener, storage: Arc<Storage>, token: String) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let storage = storage.clone();
                    let token = token.clone();
                    if let Err(e) =
                        thread::Builder::new()
                            .name("ipc-handler".into())
                            .spawn(move || {
                                Self::handle_connection(stream, &storage, &token);
                            })
                    {
                        eprintln!("[ipc] handler spawn failed: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("[ipc] accept error: {e}");
                }
            }
        }
    }

    /// Read one JSON Lines request, dispatch, write one JSON Lines response.
    fn handle_connection(stream: TcpStream, storage: &Storage, token: &str) {
        // Prevent indefinite blocking on slow/malicious clients.
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

        let peer = stream.peer_addr().ok();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        // Read exactly one request line (the CLI sends one request per connection).
        match reader.read_line(&mut line) {
            Ok(0) => return, // EOF
            Ok(_) => {}
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    eprintln!("[ipc] read timeout from {peer:?}");
                } else {
                    eprintln!("[ipc] read error from {peer:?}: {e}");
                }
                return;
            }
        }

        let line = line.trim();
        if line.is_empty() {
            return;
        }

        let request: IpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                let resp = IpcResponse::err(&IpcError::InvalidJson(e.to_string()));
                Self::write_response(&stream, &resp);
                return;
            }
        };

        if request.token.as_deref() != Some(token) {
            let resp = IpcResponse::err(&IpcError::IpcDisabled);
            Self::write_response(&stream, &resp);
            return;
        }

        let response = Self::dispatch(&request.cmd, &request.args, storage);
        Self::write_response(&stream, &response);
    }

    /// Serialize and write a single JSON Lines response.
    fn write_response(mut stream: &TcpStream, resp: &IpcResponse) {
        if let Ok(mut json) = serde_json::to_string(resp) {
            json.push('\n');
            let _ = stream.write_all(json.as_bytes());
            let _ = stream.flush();
        }
    }

    /// Route a parsed request to the appropriate handler.
    fn dispatch(cmd: &str, args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        match cmd {
            "list" => Self::cmd_list(args, storage),
            "search" => Self::cmd_search(args, storage),
            "paste" => Self::cmd_paste(args, storage),
            "pin" => Self::cmd_pin(args, storage),
            "tag" => Self::cmd_tag(args, storage),
            "delete" => Self::cmd_delete(args, storage),
            "status" => Self::cmd_status(storage),
            "add" => Self::cmd_add(args, storage),
            "snippet_list" => Self::cmd_snippet_list(storage),
            "snippet_add" => Self::cmd_snippet_add(args, storage),
            "snippet_remove" => Self::cmd_snippet_remove(args, storage),
            "snippet_insert" => Self::cmd_snippet_insert(args, storage),
            other => IpcResponse::err(&IpcError::UnknownCommand(Self::unknown_cmd_code(other))),
        }
    }

    /// Derive a stable numeric code for an unknown command name.
    fn unknown_cmd_code(name: &str) -> i32 {
        // Simple hash: sum of bytes mod 100 + 100, guaranteed > 4
        let sum: u32 = name.bytes().map(|b| b as u32).sum();
        (sum % 100 + 100) as i32
    }

    // ── Command handlers ──────────────────────────────────────────────

    /// `list` — return recent clipboard entries as summaries.
    /// Optional args: `{"kind":"text","tag":"work","query":"hello"}`
    fn cmd_list(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .map(ClipboardKind::from);
        let tag = args.get("tag").and_then(|v| v.as_str());

        match storage.list_summaries_filtered(query, kind.as_ref(), tag, None) {
            Ok(entries) => IpcResponse::ok(serde_json::to_value(entries).unwrap_or_default()),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    /// `search` — alias for `list` with `query` required.
    fn cmd_search(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        // search is semantically the same as list with a query
        Self::cmd_list(args, storage)
    }

    /// `paste` — return the full content of an entry by id.
    /// Args: `{"id":123}`
    fn cmd_paste(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                return IpcResponse::err(&IpcError::InvalidJson("missing 'id' field".into()));
            }
        };

        match storage.get_entry(id) {
            Ok(Some(entry)) => {
                let _ = storage.increment_use_count(id);
                IpcResponse::ok(serde_json::to_value(&entry).unwrap_or_default())
            }
            Ok(None) => IpcResponse::err(&IpcError::NotFound),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    /// `pin` — toggle pin state of an entry.
    /// Args: `{"id":123}`
    fn cmd_pin(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                return IpcResponse::err(&IpcError::InvalidJson("missing 'id' field".into()));
            }
        };

        match storage.toggle_pin(id) {
            Ok(()) => IpcResponse::ok(serde_json::json!({"toggled": id})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    /// `tag` — set tags on an entry.
    /// Args: `{"id":123,"tags":["work","important"]}`
    fn cmd_tag(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                return IpcResponse::err(&IpcError::InvalidJson("missing 'id' field".into()));
            }
        };

        let tags: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        match storage.set_tags(id, &tags) {
            Ok(()) => IpcResponse::ok(serde_json::json!({"tagged": id, "tags": tags})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    /// `delete` — remove an entry by id.
    /// Args: `{"id":123}`
    fn cmd_delete(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                return IpcResponse::err(&IpcError::InvalidJson("missing 'id' field".into()));
            }
        };

        match storage.delete(id) {
            Ok(()) => IpcResponse::ok(serde_json::json!({"deleted": id})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    /// `status` — return basic server status.
    fn cmd_status(storage: &Storage) -> IpcResponse {
        let entry_count = storage.list_all_summaries().map(|v| v.len()).unwrap_or(0);
        let tags = storage.saved_tags().unwrap_or_default();

        let sync_device_id = storage
            .get_setting("sync.device_id")
            .ok()
            .flatten()
            .unwrap_or_default();

        IpcResponse::ok(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "entry_count": entry_count,
            "saved_tags": tags,
            "sync": {
                "enabled": !sync_device_id.is_empty(),
                "device_id": sync_device_id,
                "state": "idle",
            },
        }))
    }

    /// `add` — create a new clipboard entry from text.
    /// Args: `{"text":"hello world"}`
    fn cmd_add(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let text = match args.get("text").and_then(|v| v.as_str()) {
            Some(t) if !t.is_empty() => t,
            _ => {
                return IpcResponse::err(&IpcError::InvalidJson(
                    "missing or empty 'text' field".into(),
                ));
            }
        };

        let entry = match ClipboardEntry::captured_text(text.to_string(), "cli".to_string()) {
            Some(e) => e,
            None => {
                return IpcResponse::err(&IpcError::InvalidJson("empty or invalid text".into()));
            }
        };
        match storage.save_entry(&entry) {
            Ok(id) => IpcResponse::ok(serde_json::json!({"id": id})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    fn cmd_snippet_list(storage: &Storage) -> IpcResponse {
        match storage.load_snippets() {
            Ok(snippets) => IpcResponse::ok(serde_json::to_value(snippets).unwrap_or_default()),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    fn cmd_snippet_add(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let name = match args.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n,
            _ => return IpcResponse::err(&IpcError::InvalidJson("missing 'name'".into())),
        };
        let template = match args.get("template").and_then(|v| v.as_str()) {
            Some(t) if !t.is_empty() => t,
            _ => return IpcResponse::err(&IpcError::InvalidJson("missing 'template'".into())),
        };
        let mut snippet = crate::snippets::Snippet::new(name, template);
        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
            snippet.description = desc.to_string();
        }
        match storage.save_snippet(&snippet) {
            Ok(id) => IpcResponse::ok(serde_json::json!({"id": id})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    fn cmd_snippet_remove(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return IpcResponse::err(&IpcError::InvalidJson(
                            "missing 'id' or 'name'".into(),
                        ));
                    }
                };
                match storage.load_snippets() {
                    Ok(snippets) => match snippets.iter().find(|s| s.name == name) {
                        Some(s) => s.id,
                        None => return IpcResponse::err(&IpcError::NotFound),
                    },
                    Err(e) => return IpcResponse::err(&IpcError::Storage(e.to_string())),
                }
            }
        };
        match storage.delete_snippet(id) {
            Ok(()) => IpcResponse::ok(serde_json::json!({"deleted": id})),
            Err(e) => IpcResponse::err(&IpcError::Storage(e.to_string())),
        }
    }

    fn cmd_snippet_insert(args: &serde_json::Value, storage: &Storage) -> IpcResponse {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => {
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return IpcResponse::err(&IpcError::InvalidJson(
                            "missing 'id' or 'name'".into(),
                        ));
                    }
                };
                match storage.load_snippets() {
                    Ok(snippets) => match snippets.iter().find(|s| s.name == name) {
                        Some(s) => s.id,
                        None => return IpcResponse::err(&IpcError::NotFound),
                    },
                    Err(e) => return IpcResponse::err(&IpcError::Storage(e.to_string())),
                }
            }
        };
        let snippets = match storage.load_snippets() {
            Ok(s) => s,
            Err(e) => return IpcResponse::err(&IpcError::Storage(e.to_string())),
        };
        let snippet = match snippets.iter().find(|s| s.id == id) {
            Some(s) => s.clone(),
            None => return IpcResponse::err(&IpcError::NotFound),
        };
        let vars: std::collections::HashMap<String, String> = args
            .get("var")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let mut all_vars = crate::snippets::interpolate::resolve_builtins(None, None);
        all_vars.extend(vars);
        match crate::snippets::interpolate::interpolate(&snippet.template, &all_vars) {
            Ok(text) => {
                let _ = storage.increment_snippet_use_count(id);
                IpcResponse::ok(serde_json::json!({"text": text}))
            }
            Err(e) => IpcResponse::err(&IpcError::InvalidJson(e.to_string())),
        }
    }
}

// ── Client helper (for dzc-slim) ─────────────────────────────────────

/// Send a single JSON Lines request to the IPC server and return the
/// parsed response. This is a convenience function for `dzc-slim`.
///
/// Reads the port number and token from `port_file`, connects to 127.0.0.1:port,
/// sends the authenticated request, and reads the response.
pub fn send_request(port_file: &Path, request: &IpcRequest) -> Result<IpcResponse, IpcError> {
    let endpoint = IpcEndpoint::read(port_file)?;

    let addr = format!("127.0.0.1:{}", endpoint.port);
    let stream = TcpStream::connect(&addr).map_err(|_| IpcError::ConnectionRefused)?;

    // Write request.
    let mut writer = &stream;
    let mut authenticated = request.clone();
    authenticated.token = Some(endpoint.token);
    let mut json =
        serde_json::to_string(&authenticated).map_err(|e| IpcError::InvalidJson(e.to_string()))?;
    json.push('\n');
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    // Read response.
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        return Err(IpcError::InvalidJson("empty response".into()));
    }

    serde_json::from_str::<IpcResponse>(line).map_err(|e| IpcError::InvalidJson(e.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IpcEndpoint {
    port: u16,
    token: String,
}

impl IpcEndpoint {
    fn read(path: &Path) -> Result<Self, IpcError> {
        let raw = std::fs::read_to_string(path).map_err(IpcError::Io)?;
        let mut lines = raw.lines();
        let port = lines
            .next()
            .ok_or_else(|| IpcError::InvalidJson("missing IPC port".into()))?
            .trim()
            .parse()
            .map_err(|_| IpcError::InvalidJson("invalid port in port file".into()))?;
        let token = lines
            .next()
            .ok_or_else(|| IpcError::InvalidJson("missing IPC token".into()))?
            .trim()
            .to_string();
        if token.is_empty() {
            return Err(IpcError::InvalidJson("empty IPC token".into()));
        }
        Ok(Self { port, token })
    }
}

fn new_session_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_error_exit_codes() {
        assert_eq!(IpcError::ConnectionRefused.exit_code(), 2);
        assert_eq!(IpcError::InvalidJson("x".into()).exit_code(), 3);
        assert_eq!(IpcError::UnknownCommand(42).exit_code(), 42);
        assert_eq!(IpcError::NotFound.exit_code(), 5);
        assert_eq!(IpcError::IpcDisabled.exit_code(), 6);
        assert_eq!(IpcError::Timeout.exit_code(), 1);
        // Io maps to 1
        let io_err = IpcError::Io(std::io::Error::other("test"));
        assert_eq!(io_err.exit_code(), 1);
        assert_eq!(IpcError::Storage("x".into()).exit_code(), 1);
    }

    #[test]
    fn ipc_error_display() {
        let e = IpcError::ConnectionRefused;
        assert!(e.to_string().contains("deziroslim"));

        let e = IpcError::InvalidJson("bad input".into());
        assert!(e.to_string().contains("bad input"));

        let e = IpcError::UnknownCommand(7);
        assert!(e.to_string().contains('7'));

        let e = IpcError::NotFound;
        assert!(e.to_string().contains("not found"));

        let e = IpcError::IpcDisabled;
        assert!(e.to_string().contains("disabled"));
    }

    #[test]
    fn ipc_request_deserialize() {
        let req: IpcRequest =
            serde_json::from_str(r#"{"cmd":"list","args":{"kind":"text"}}"#).unwrap();
        assert_eq!(req.cmd, "list");
        assert_eq!(req.args["kind"], "text");
        assert_eq!(req.token, None);
    }

    #[test]
    fn ipc_endpoint_reads_port_and_token() {
        let dir = tempfile::tempdir().unwrap();
        let port_file = dir.path().join("test.port");
        std::fs::write(&port_file, "12345\nabcdef\n").unwrap();

        let endpoint = IpcEndpoint::read(&port_file).unwrap();

        assert_eq!(endpoint.port, 12345);
        assert_eq!(endpoint.token, "abcdef");
    }

    #[test]
    fn ipc_endpoint_rejects_missing_token() {
        let dir = tempfile::tempdir().unwrap();
        let port_file = dir.path().join("test.port");
        std::fs::write(&port_file, "12345\n").unwrap();

        let err = IpcEndpoint::read(&port_file).unwrap_err();

        assert!(matches!(err, IpcError::InvalidJson(_)));
    }

    #[test]
    fn ipc_request_missing_args_defaults() {
        let req: IpcRequest = serde_json::from_str(r#"{"cmd":"status"}"#).unwrap();
        assert_eq!(req.cmd, "status");
        assert_eq!(req.args, serde_json::Value::Null);
        assert_eq!(req.token, None);
    }

    #[test]
    fn ipc_response_success_serializes() {
        let resp = IpcResponse::ok(serde_json::json!({"count": 42}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""ok":true"#));
        assert!(json.contains(r#""count":42"#));
        assert!(!json.contains("error"));
    }

    #[test]
    fn ipc_response_error_serializes() {
        let resp = IpcResponse::err(&IpcError::NotFound);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""ok":false"#));
        assert!(json.contains(r#""code":5"#));
        assert!(!json.contains(r#""data""#));
    }

    #[test]
    fn ipc_error_body_roundtrip() {
        let body = IpcErrorBody {
            code: 3,
            message: "bad json".into(),
        };
        let json = serde_json::to_string(&body).unwrap();
        let parsed: IpcErrorBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, 3);
        assert_eq!(parsed.message, "bad json");
    }

    #[test]
    fn port_file_default_uses_xdg_runtime_on_linux() {
        #[cfg(target_os = "linux")]
        {
            let dir = tempfile::tempdir().expect("create tempdir");
            let runtime = dir.path().to_str().expect("utf8 path").to_string();
            let expected = dir.path().join("deziroslim.port");

            let old = std::env::var("XDG_RUNTIME_DIR").ok();

            // SAFETY: test runs single-threaded in a temp context.
            unsafe {
                std::env::set_var("XDG_RUNTIME_DIR", &runtime);
            }
            let path = IpcServer::port_file_default();
            assert_eq!(path, expected);

            match old {
                Some(v) => unsafe { std::env::set_var("XDG_RUNTIME_DIR", v) },
                None => unsafe { std::env::remove_var("XDG_RUNTIME_DIR") },
            }
        }
    }

    #[test]
    fn port_file_default_fallback() {
        #[cfg(target_os = "linux")]
        let old = std::env::var("XDG_RUNTIME_DIR").ok();
        #[cfg(target_os = "linux")]
        // SAFETY: test runs single-threaded in a temp context.
        unsafe {
            std::env::remove_var("XDG_RUNTIME_DIR");
        }

        let path = IpcServer::port_file_default();
        // Should end with deziroslim/port
        assert!(path.ends_with("deziroslim/port") || path.ends_with("deziroslim\\port"));

        #[cfg(target_os = "linux")]
        {
            if let Some(v) = old {
                unsafe {
                    std::env::set_var("XDG_RUNTIME_DIR", v);
                }
            }
        }
    }

    #[test]
    fn unknown_cmd_code_is_deterministic() {
        let code1 = IpcServer::unknown_cmd_code("foobar");
        let code2 = IpcServer::unknown_cmd_code("foobar");
        assert_eq!(code1, code2);
        // Must be >= 100 (guaranteed by implementation)
        assert!(code1 >= 100);
    }

    #[test]
    fn roundtrip_request_response_via_mock() {
        // Simulate the serialization path without a real socket.
        let req = IpcRequest {
            cmd: "list".into(),
            args: serde_json::json!({"query": "test"}),
            token: None,
        };
        let req_json = serde_json::to_string(&req).unwrap();
        let parsed: IpcRequest = serde_json::from_str(&req_json).unwrap();
        assert_eq!(parsed.cmd, "list");

        let resp = IpcResponse::ok(serde_json::json!({"entries": []}));
        let resp_json = serde_json::to_string(&resp).unwrap();
        let parsed_resp: IpcResponse = serde_json::from_str(&resp_json).unwrap();
        assert!(parsed_resp.ok);
        assert!(parsed_resp.error.is_none());
    }

    #[test]
    fn server_start_and_status_roundtrip() {
        use std::time::Duration;

        // Create a temporary storage.
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_ipc.db");
        let storage = Arc::new(Storage::open(db_path).unwrap());
        storage.cleanup_expired().unwrap();

        let port_file = dir.path().join("test.port");
        let server = IpcServer::start(storage, port_file.clone()).unwrap();

        // Give the server thread a moment to bind.
        std::thread::sleep(Duration::from_millis(50));

        // Send a status request.
        let req = IpcRequest {
            cmd: "status".into(),
            args: serde_json::Value::Null,
            token: None,
        };
        let resp = send_request(&server.port_file, &req).unwrap();
        assert!(resp.ok);
        let data = resp.data.unwrap();
        assert_eq!(data["version"], env!("CARGO_PKG_VERSION"));
        assert!(data["entry_count"].as_u64().is_some());

        // Cleanup.
        let _ = std::fs::remove_file(&port_file);
    }

    #[test]
    fn server_rejects_request_without_session_token() {
        use std::net::TcpStream;
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_ipc_auth.db");
        let storage = Arc::new(Storage::open(db_path).unwrap());
        storage.cleanup_expired().unwrap();

        let port_file = dir.path().join("test_auth.port");
        let server = IpcServer::start(storage, port_file.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let endpoint = IpcEndpoint::read(&server.port_file).unwrap();
        let stream = TcpStream::connect(("127.0.0.1", endpoint.port)).unwrap();
        let request = IpcRequest {
            cmd: "status".into(),
            args: serde_json::Value::Null,
            token: None,
        };
        let mut writer = &stream;
        let mut json = serde_json::to_string(&request).unwrap();
        json.push('\n');
        writer.write_all(json.as_bytes()).unwrap();
        writer.flush().unwrap();

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let response: IpcResponse = serde_json::from_str(line.trim()).unwrap();

        assert!(!response.ok);
        assert_eq!(
            response.error.unwrap().code,
            IpcError::IpcDisabled.exit_code()
        );
        let _ = std::fs::remove_file(&port_file);
    }

    #[test]
    fn server_list_empty() {
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_ipc_list.db");
        let storage = Arc::new(Storage::open(db_path).unwrap());
        storage.cleanup_expired().unwrap();

        let port_file = dir.path().join("test_list.port");
        let server = IpcServer::start(storage, port_file.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let req = IpcRequest {
            cmd: "list".into(),
            args: serde_json::Value::Null,
            token: None,
        };
        let resp = send_request(&server.port_file, &req).unwrap();
        assert!(resp.ok);
        let entries = resp.data.unwrap();
        assert!(entries.as_array().unwrap().is_empty());

        let _ = std::fs::remove_file(&port_file);
    }

    #[test]
    fn server_unknown_command() {
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_ipc_unk.db");
        let storage = Arc::new(Storage::open(db_path).unwrap());
        storage.cleanup_expired().unwrap();

        let port_file = dir.path().join("test_unk.port");
        let server = IpcServer::start(storage, port_file.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let req = IpcRequest {
            cmd: "nonexistent".into(),
            args: serde_json::Value::Null,
            token: None,
        };
        let resp = send_request(&server.port_file, &req).unwrap();
        assert!(!resp.ok);
        let err = resp.error.unwrap();
        assert!(err.code >= 100); // unknown command codes >= 100
        assert!(err.message.contains("unknown"));

        let _ = std::fs::remove_file(&port_file);
    }

    #[test]
    fn server_delete_not_found() {
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_ipc_del.db");
        let storage = Arc::new(Storage::open(db_path).unwrap());
        storage.cleanup_expired().unwrap();

        let port_file = dir.path().join("test_del.port");
        let server = IpcServer::start(storage, port_file.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // Deleting non-existent id should succeed (SQLite DELETE is idempotent).
        let req = IpcRequest {
            cmd: "delete".into(),
            args: serde_json::json!({"id": 999999}),
            token: None,
        };
        let resp = send_request(&server.port_file, &req).unwrap();
        assert!(resp.ok);

        let _ = std::fs::remove_file(&port_file);
    }
}
