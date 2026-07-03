//! deziroslim library crate.
//!
//! Exposes the GUI app modules so that the `dzc-slim` binary can use the
//! shared data model and storage. The GUI binary (`main.rs`) uses these
//! modules via `use deziroslim::*`.

rust_i18n::i18n!("locales", fallback = "en-US");

pub mod actions;
pub mod app;
pub mod backup;
pub mod blacklist;
pub mod clipboard;
pub mod emoji_data;
pub mod encryption;
pub mod i18n;
pub mod ipc;
pub mod model;
pub mod platform;
pub mod search;
pub mod snippets;
pub mod sound;
pub mod storage;
pub mod storage_io;
pub mod sync;
pub mod ui;
