pub mod commands;
pub mod executor;
#[cfg(target_os = "macos")]
pub mod launchctl;
pub mod logs;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
pub mod models;
pub mod paths;
#[cfg(target_os = "macos")]
pub mod plist_writer;
pub mod registry;
pub mod runtime;
#[cfg(target_os = "windows")]
pub(crate) mod task_scheduler;
#[cfg(any(target_os = "windows", test))]
mod task_xml;
pub mod validation;

#[cfg(target_os = "macos")]
pub(crate) use macos as platform;
#[cfg(target_os = "windows")]
pub(crate) use task_scheduler as platform;
