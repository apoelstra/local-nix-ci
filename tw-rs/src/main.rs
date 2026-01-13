// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod repo;

use std::process::Command;

fn check_required_tools() {
    let tools = ["gh", "git", "jj", "nix", "task"];
    
    for tool in &tools {
        match Command::new(tool).arg("--version").output() {
            Ok(output) if output.status.success() => {
                // Tool is available
            }
            _ => {
                eprintln!("Error: Required tool '{}' is not available or not in PATH", tool);
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    // Parse CLI arguments -- if this fails it will just terminate the program
    // with a usage message.
    args::parse_cli();

    // Check that all required tools are available
    check_required_tools();

    println!("Hello, world!");
}
