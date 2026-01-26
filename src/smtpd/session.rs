// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/session.rs
// Session of SMTPd.

use super::cmd;
use crate::{conf, constants};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;

pub enum SmtpSessionStatus {
    Init,
    Hello,
    Sender,
    Rcpt,
    Data,
}

pub struct SmtpSession {
    pub config: Arc<conf::ConfigSmtpServer>,
    pub addr: SocketAddr,
    pub reader: cmd::LineReader,
    pub writer: OwnedWriteHalf,
    pub status: SmtpSessionStatus,
    pub tls: bool,
    pub client: String,
}

impl SmtpSession {
    pub fn new(config: Arc<conf::ConfigSmtpServer>, addr: SocketAddr, stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        SmtpSession {
            config,
            addr,
            reader: cmd::LineReader::Stream(cmd::StreamReader::new(BufReader::new(reader))),
            writer,
            status: SmtpSessionStatus::Init,
            tls: false,
            client: String::new(),
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while self.start().await? {}
        Ok(())
    }

    async fn start(&mut self) -> anyhow::Result<bool> {
        let hello = format!(
            "220 {} {} {}\r\n",
            self.config.domain,
            constants::SMTPD_INFO,
            constants::SMTPD_NAME
        );
        self.status = SmtpSessionStatus::Init;
        self.writer.write_all(hello.as_bytes()).await?;
        cmd::run(self).await
    }
}
