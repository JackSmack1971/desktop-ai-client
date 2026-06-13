/// Desktop AI Client – Tauri backend library crate.
///
/// This crate is the Rust backend for the desktop AI client. It is compiled as
/// both a `staticlib`/`cdylib` (for the Tauri runtime) and an `rlib` (for Rust
/// unit tests).
///
/// Privacy invariant: nothing in this crate may expose provider credentials, raw
/// file paths, or prompt content to the frontend. The Rust boundary is the trust
/// line for all sensitive operations.
pub mod app_state;
pub mod ipc;
pub mod providers;
pub mod security;
pub mod storage;
pub mod telemetry;
