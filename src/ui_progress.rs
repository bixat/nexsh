use indicatif::{ProgressBar, ProgressStyle};
use std::borrow::Cow;
use std::time::Duration;

pub struct ProgressManager;

impl ProgressManager {
    pub fn new() -> Self {
        Self
    }

    /// Creates and configures a spinner progress bar with a colored message
    pub fn create_spinner(&self, message: impl Into<Cow<'static, str>>) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        let spinner_style = ProgressStyle::with_template("{spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

        pb.set_style(spinner_style);
        pb.enable_steady_tick(Duration::from_millis(30));
        pb.set_message(message);
        pb
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}
