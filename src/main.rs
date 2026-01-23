// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/main.rs
// Entry point.

mod conf;

use std::sync::Arc;

fn main() {
    init("example/arcmail.json");
}

fn init(path: &str) {
    let config = conf::Config::load_path(path);

    print!("{:?}", config);
    println!("!");
}
