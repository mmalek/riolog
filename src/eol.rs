#[cfg(target_os = "windows")]
pub const EOL: &[u8] = b"\r\n";

#[cfg(not(target_os = "windows"))]
pub const EOL: &[u8] = b"\n";
