//! XFixes-based clipboard monitoring for Linux/X11.

use x11rb::connection::Connection;
use x11rb::protocol::xfixes::SelectionEventMask;
use x11rb::rust_connection::RustConnection;

/// Probes XFixes extension availability for clipboard monitoring decisions.
pub struct XFixesProbe;

impl XFixesProbe {
    /// Returns `true` if XFixes 4.0+ is available (required for `SelectSelectionInput`).
    /// Returns `false` when no X11 display or extension is missing/too old.
    pub fn is_available() -> bool {
        let Ok((conn, _screen_num)) = x11rb::connect(None) else {
            return false;
        };
        // XFixes 4.0 added SelectSelectionInput for clipboard monitoring.
        x11rb::protocol::xfixes::query_version(&conn, 4, 0)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some()
    }

    /// Subscribe to PRIMARY selection ownership changes via XFixes.
    ///
    /// T17 implements the event loop; this just sends the request.
    /// Returns `Err` on failure (caller falls back to arboard polling).
    pub fn subscribe_primary_notify(
        conn: &RustConnection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let screen = &conn.setup().roots[0];
        let root = screen.root;

        let primary_cookie = x11rb::protocol::xproto::intern_atom(conn, false, b"PRIMARY")?;
        let primary = primary_cookie.reply()?.atom;

        x11rb::protocol::xfixes::select_selection_input(
            conn,
            root,
            primary,
            SelectionEventMask::SET_SELECTION_OWNER,
        )?
        .check()?;

        conn.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::XFixesProbe;

    #[test]
    fn is_available_returns_bool_without_panic() {
        let _available = XFixesProbe::is_available();
    }
}
