// SPDX-License-Identifier: GPL-3.0-or-later

mod args;

fn main() {
    // Parse CLI arguments -- if this fails it will just terminate the program
    // with a usage message.
    args::parse_cli();

    println!("Hello, world!");
}
