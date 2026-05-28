mod config;
mod varlink_monitor;

use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::Context;
use log::error;

use crate::{config::load_rules, nft::nftset_add};

mod error;
mod nft;

const DEFAULT_CONFIG_PATH: &str = "/etc/resolved-nftset";
const RESOLVED_MONITOR_SOCKET: &str = "/run/systemd/resolve/io.systemd.Resolve.Monitor";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_path = std::env::var("CONF_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CONFIG_PATH));

    let rules = load_rules(&config_path).with_context(|| "Failed to load config")?;

    let stream = UnixStream::connect(RESOLVED_MONITOR_SOCKET)
        .with_context(|| "Failed to connect to systemd-resolved unix socket")?;

    varlink_monitor::subscribe(stream, |hostname, addrs_v4, addrs_v6| {
        for ruleset in &rules {
            if !ruleset.matches(&hostname) {
                continue;
            }
            if let Err(e) = nftset_add(
                &ruleset.table_name,
                &ruleset.set_name_v4,
                &ruleset.set_name_v6,
                &addrs_v4,
                &addrs_v6,
            ) {
                error!(
                    "Failed to add to the nft set '{}.{}': {}",
                    ruleset.table_name,
                    &ruleset.set_name_v4[..ruleset.set_name_v4.len() - 3],
                    e
                );
            }
        }
    })?;

    Ok(())
}
