rust_i18n::i18n!("locales", fallback = "en-US");

use anyhow::Context;
use app::ClipboardApp;
use ipc::IpcServer;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use storage::Storage;
use tiez_slim_linux::*;

const APP_DISPLAY_NAME: &str = "tiez-slim";
const APP_ID: &str = "tiez-slim-linux";
const DB_PATH_ENV: &str = "TIEZ_SLIM_LINUX_DB_PATH";
const DEV_MODE_ENV: &str = "TIEZ_SLIM_LINUX_DEV";
const LEGACY_DB_PATH_ENV: &str = "MYCLIPBOARD_DB_PATH";
const LEGACY_DEV_MODE_ENV: &str = "MYCLIPBOARD_DEV";

fn main() -> anyhow::Result<()> {
    if let Some(command) = dev_command() {
        return run_dev_command(command);
    }

    #[cfg(feature = "log-miss-tr")]
    env_logger::init();

    let dev_mode = dev_mode_enabled();
    let minimized = minimized_start_enabled();
    #[allow(unused_mut)]
    let mut storage =
        Storage::open(resolve_db_path()).context(rust_i18n::t!("error.open_db_failed"))?;
    storage
        .cleanup_expired()
        .context(rust_i18n::t!("error.cleanup_failed"))?;

    let ipc_socket = IpcServer::socket_path_default();
    let ipc_storage = Arc::new(storage.clone());
    match IpcServer::start(ipc_storage, ipc_socket) {
        Ok(_server) => {}
        Err(e) => eprintln!("IPC server failed to start: {e}"),
    }

    let mut _migration_queue: Option<encryption::queue::MigrationQueue> = None;
    #[cfg(feature = "secure_storage")]
    match encryption::KeyringBackend::new() {
        Ok(backend) => {
            let enc: Arc<dyn encryption::SecureStore + Send + Sync> = Arc::new(backend);
            storage.set_encryptor(enc.clone());
            _migration_queue = Some(encryption::queue::MigrationQueue::start(
                storage.clone(),
                enc,
            ));
        }
        Err(e) => eprintln!("encryption unavailable, storing plaintext: {e}"),
    }

    let data_dir = storage
        .path()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let backup_storage = storage.clone();
    let _ = ctrlc::set_handler(move || {
        let _ = backup::AutoBackup::new(data_dir.clone(), 10).run_backup(&backup_storage);
        std::process::exit(0);
    });

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(APP_DISPLAY_NAME)
        .with_inner_size([480.0, 680.0])
        .with_min_inner_size([320.0, 400.0])
        .with_position(initial_window_position())
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(true)
        .with_visible(!minimized);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(move |cc| {
            Ok(Box::new(ClipboardApp::new(
                cc, storage, dev_mode, !minimized,
            )))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DevCommand {
    Ci,
}

fn dev_command() -> Option<DevCommand> {
    dev_command_from_args(std::env::args())
}

fn dev_command_from_args<I, S>(args: I) -> Option<DevCommand>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match args.into_iter().nth(1).as_ref().map(AsRef::as_ref) {
        Some("ci") => Some(DevCommand::Ci),
        _ => None,
    }
}

fn run_dev_command(command: DevCommand) -> anyhow::Result<()> {
    match command {
        DevCommand::Ci => run_ci(),
    }
}

fn run_ci() -> anyhow::Result<()> {
    let script = std::path::Path::new("scripts/i18n-check.sh");
    if !script.exists() {
        anyhow::bail!("`scripts/i18n-check.sh` not found; run `cargo ci` from the repository root");
    }

    // The i18n script uses `set -o pipefail` and `#!/usr/bin/env bash`, so it
    // must be executed with bash. Fall back to sh only as a last resort so the
    // i18n step runs on minimal environments; on shells without pipefail the
    // step will fail loudly, which is the correct behavior.
    let script_interpreter = if which("bash").is_some() {
        "bash"
    } else if which("sh").is_some() {
        "sh"
    } else {
        anyhow::bail!("neither `bash` nor `sh` is available in PATH");
    };

    let steps: &[(&str, &[&str])] = &[
        ("cargo", &["fmt", "--all", "--", "--check"]),
        ("cargo", &["check"]),
        ("cargo", &["test"]),
        (
            "cargo",
            &["clippy", "--all-targets", "--", "-D", "warnings"],
        ),
        (script_interpreter, &["scripts/i18n-check.sh"]),
    ];

    for (program, args) in steps {
        println!("$ {} {}", program, args.join(" "));
        let status = Command::new(program)
            .args(*args)
            .status()
            .with_context(|| format!("failed to start `{}`", format_command(program, args)))?;
        if !status.success() {
            anyhow::bail!(
                "`{}` failed with status {status}",
                format_command(program, args)
            );
        }
    }

    println!("ci passed");
    Ok(())
}

fn which(program: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    })
}

fn format_command(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

fn minimized_start_enabled() -> bool {
    std::env::args().skip(1).any(|arg| arg == "--minimized")
}

fn initial_window_position() -> egui::Pos2 {
    let screen = platform::screen_geometry().unwrap_or(platform::ScreenGeometry {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 800.0,
    });
    egui::pos2(
        screen.x + ((screen.width - 480.0) / 2.0).max(8.0),
        screen.y + ((screen.height - 680.0) / 2.0).max(8.0),
    )
}

fn resolve_db_path() -> PathBuf {
    parse_db_path_from_args()
        .or_else(|| std::env::var(DB_PATH_ENV).ok().map(PathBuf::from))
        .or_else(|| std::env::var(LEGACY_DB_PATH_ENV).ok().map(PathBuf::from))
        .or_else(Storage::path_from_redirect_file)
        .unwrap_or_else(Storage::default_path)
}

fn parse_db_path_from_args() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--db-path" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn dev_mode_enabled() -> bool {
    let flag_enabled = dev_mode_arg_enabled(std::env::args().skip(1));
    let env_enabled = std::env::var(DEV_MODE_ENV)
        .or_else(|_| std::env::var(LEGACY_DEV_MODE_ENV))
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    flag_enabled || env_enabled || cfg!(feature = "devtools")
}

fn dev_mode_arg_enabled<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .any(|arg| matches!(arg.as_ref(), "--dev" | "dev"))
}

fn load_window_icon() -> Option<Arc<egui::IconData>> {
    let image = image::load_from_memory(include_bytes!("../assets/icons/tiez-slim-linux.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Some(Arc::new(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_arg_is_dev_command() {
        assert_eq!(
            dev_command_from_args(["tiez-slim-linux", "ci"]),
            Some(DevCommand::Ci)
        );
    }

    #[test]
    fn dev_arg_enables_dev_mode() {
        assert!(dev_mode_arg_enabled(["dev"]));
        assert!(dev_mode_arg_enabled(["--dev"]));
        assert!(!dev_mode_arg_enabled(["--minimized"]));
    }
}
