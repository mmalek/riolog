#[cfg(target_os = "windows")]
pub const EOL: &[u8] = b"\r\n";

#[cfg(target_os = "windows")]
pub const EOL_REVERSED: &[u8] = b"\n\r";

#[cfg(not(target_os = "windows"))]
pub const EOL: &[u8] = b"\n";

#[cfg(not(target_os = "windows"))]
pub const EOL_reversed: &[u8] = b"\n";
