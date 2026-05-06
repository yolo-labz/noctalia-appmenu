//! Manual probe: walk the AT-SPI menubar for a given PID and print
//! the JSON tree. Used for live verification during v0.3 substrate
//! bring-up — not part of the production daemon.
//!
//! Usage:
//!   cargo run --example `atspi_probe` -- <pid>

use anyhow::Result;
use noctalia_appmenu_bridge::atspi;

#[tokio::main]
async fn main() -> Result<()> {
    let pid: u32 = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: atspi_probe <pid>"))?
        .parse()?;
    atspi::enable_a11y().await?;
    match atspi::fetch_menubar_for_pid(pid).await? {
        Some(tree) => println!("{}", serde_json::to_string_pretty(&tree)?),
        None => println!("null"),
    }
    Ok(())
}
