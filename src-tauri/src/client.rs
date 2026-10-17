#[cfg(windows)]
#[path = "client_windows.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "client_macos.rs"]
mod platform;

pub use platform::{find_client, launch_client, stop_client};