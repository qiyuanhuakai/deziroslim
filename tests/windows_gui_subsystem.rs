#![cfg(target_os = "windows")]

use std::fs;

const PE_HEADER_OFFSET: usize = 0x3c;
const OPTIONAL_HEADER_OFFSET: usize = 24;
const SUBSYSTEM_OFFSET: usize = 68;
const WINDOWS_GUI_SUBSYSTEM: u16 = 2;

#[test]
fn deziroslim_binary_uses_windows_gui_subsystem() {
    let binary = fs::read(env!("CARGO_BIN_EXE_deziroslim")).expect("read deziroslim binary");
    let pe_offset = u32::from_le_bytes(
        binary[PE_HEADER_OFFSET..PE_HEADER_OFFSET + 4]
            .try_into()
            .expect("read PE header offset"),
    ) as usize;
    let subsystem_offset = pe_offset + OPTIONAL_HEADER_OFFSET + SUBSYSTEM_OFFSET;
    let subsystem = u16::from_le_bytes(
        binary[subsystem_offset..subsystem_offset + 2]
            .try_into()
            .expect("read PE subsystem"),
    );

    assert_eq!(subsystem, WINDOWS_GUI_SUBSYSTEM);
}
