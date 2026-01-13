// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod repo;

use xshell::{Shell, cmd};

fn check_required_tools() -> xshell::Result<()> {
    let sh = Shell::new()?;
    let tools = ["gh", "git", "jj", "nix", "task"];
    
    for tool in &tools {
        if cmd!(sh, "{tool} --version").quiet().run().is_err() {
            eprintln!("Error: Required tool '{}' is not available or not in PATH", tool);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn main() {
    // Parse CLI arguments -- if this fails it will just terminate the program
    // with a usage message.
    args::parse_cli();

    // Check that all required tools are available
    if let Err(e) = check_required_tools() {
        eprintln!("Error checking required tools: {}", e);
        std::process::exit(1);
    }

    println!("Hello, world!");
}
