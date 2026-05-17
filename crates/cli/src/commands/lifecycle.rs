//! Per-collection MCP server lifecycle: start / stop / restart. Delegates to
//! the fleet registry; the heavy lifting lives in `crate::fleet`.

use std::path::Path;

use crate::fleet;

pub fn cmd_start(name: &str, config_path: &Path) -> Result<(), String> {
    let reg = fleet::Registry::open(&fleet::registry_path())?;
    let msg = fleet::start(&reg, name, config_path)?;
    println!("{msg}");
    Ok(())
}

pub fn cmd_stop(name: &str) -> Result<(), String> {
    let reg = fleet::Registry::open(&fleet::registry_path())?;
    let msg = fleet::stop(&reg, name)?;
    println!("{msg}");
    Ok(())
}

pub fn cmd_restart(name: &str, config_path: &Path) -> Result<(), String> {
    let reg = fleet::Registry::open(&fleet::registry_path())?;
    let msg = fleet::restart(&reg, name, config_path)?;
    println!("{msg}");
    Ok(())
}
