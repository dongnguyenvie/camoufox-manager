//! CLI — phase 12 implements it: list/create/start/stop/delete profiles and proxies against
//! the local API (`http://127.0.0.1:$CM_PORT`).

use anyhow::{Result, bail};

pub fn run(args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") => {
            println!("usage: camoufox-manager            open the desktop window");
            println!("       camoufox-manager serve      run the server only (CM_PORT, default {})", crate::DEFAULT_PORT);
            Ok(())
        }
        Some(cmd) => bail!("unknown command `{cmd}` (the CLI is not implemented yet; try `help`)"),
        None => Ok(()),
    }
}
