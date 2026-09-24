//! 🍅 Tomato Clock — a straightforward command-line Pomodoro timer.
//!
//! The binary is a thin wrapper around [`app::run`]; everything else lives
//! here so it can be unit- and integration-tested.

pub mod app;
pub mod cli;
pub mod notify;
pub mod progress;
pub mod timer;
