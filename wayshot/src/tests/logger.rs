use clap::Parser;

use crate::cli::Cli;
use crate::config::Config;

// `tracing_subscriber::fmt().init()` installs a *global* default subscriber
// and panics if one is already set. This must stay the only test in the
// whole suite that calls `logger::setup`.
#[test]
fn setup_installs_a_global_subscriber_without_panicking() {
    let cli = Cli::parse_from(["wayshot", "--log-level", "debug"]);
    let config = Config::default();
    crate::logger::setup(&cli, &config);
    tracing::debug!("logger smoke test");
}
