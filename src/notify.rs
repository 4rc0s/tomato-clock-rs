use anyhow::Context;

/// Send a desktop notification. Failures are returned so the caller can
/// warn without aborting the timer.
pub fn notify(msg: &str) -> anyhow::Result<()> {
    notify_rust::Notification::new()
        .summary("🍅 Tomato Clock")
        .body(msg)
        .appname("tomato")
        .show()
        .context("failed to send desktop notification")?;
    Ok(())
}
