// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/conf.rs
// Manage configure file.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSmtpServerExpand {
    pub pipe_len: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSmtpServer {
    pub domain: String,
    pub binds: Vec<String>,
    pub ports: Vec<u16>,
    pub hello: String,
    pub expand: ConfigSmtpServerExpand,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub smtp_servers: Vec<Arc<ConfigSmtpServer>>,
}

impl Config {
    pub fn load_path(path: &str) -> anyhow::Result<Config> {
        let file = File::open(path)?;
        Self::load_reader(file)
    }

    pub fn load_reader<R: std::io::Read>(mut reader: R) -> anyhow::Result<Config> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(serde_json::from_slice(&buf)?)
    }

    pub fn save_path(&self, path: &str) -> anyhow::Result<()> {
        let tmp = format!("{}.tmp", path);
        let mut file = File::create(&tmp)?;
        file.write_all(serde_json::to_vec_pretty(self)?.as_slice())?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}
