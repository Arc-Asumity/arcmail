// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/manager.rs
// Manager Thread.

use crate::smtpd::server;
use tokio::sync::watch;

trait Control {
    fn stop(&self);
}

pub struct ServerControl {
    pub smtpds_control: Vec<watch::Sender<server::SmtpServerControl>>,
}

impl ServerControl {
    pub async fn stop(self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct ServerList {
    pub smtpds: Vec<server::SmtpServer>,
}
