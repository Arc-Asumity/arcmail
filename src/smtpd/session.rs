// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/session.rs
// Session of SMTPd.

use super::{cmd, stream, util};
use crate::{allow, conf, constants};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;

pub enum SmtpSessionStatus {
    Start,
    Init,
    Hello,
    Sender,
    Rcpt,
    Data,
    Stop,
}

pub struct SmtpSession {
    pub config: Arc<conf::ConfigSmtpServer>,
    pub addr: SocketAddr,
    pub stream: stream::SmtpStream,
    pub status: SmtpSessionStatus,
    pub tls: bool,
    pub client: String,
}

impl SmtpSession {
    pub fn new(config: Arc<conf::ConfigSmtpServer>, addr: SocketAddr, stream: TcpStream) -> Self {
        let tx_len = config.expand.pipe_tx_len;
        let rx_len = config.expand.pipe_rx_len;
        SmtpSession {
            config,
            addr,
            stream: stream::SmtpStream::Tcp(stream::TcpSmtpStream::new(stream, tx_len, rx_len)),
            status: SmtpSessionStatus::Start,
            tls: false,
            client: String::new(),
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while !matches!(self.status, SmtpSessionStatus::Stop) {
            if matches!(self.status, SmtpSessionStatus::Start) {
                let hello = format!(
                    "220 {} {} {}\r\n",
                    self.config.domain,
                    constants::SMTPD_INFO,
                    constants::SMTPD_NAME
                );
                self.stream.get_writer().send(hello).await?;
                self.status = SmtpSessionStatus::Init;
            }
            let mut mes = match self.stream.read_line().await {
                Some(s) => s,
                None => break,
            };
            util::remove_crlf(&mut mes);
            let para: Vec<&str> = mes.split_whitespace().collect();
            if mes.is_empty() {
                allow::SmtpError::new(500)
                    .return_code(self.stream.get_writer())
                    .await?;
                continue;
            }
            if cmd::global_command(&mut self, &para).await? {
                continue;
            }
            allow::check_command(&para[0])
                .return_code(self.stream.get_writer())
                .await?;
        }
        Ok(())
    }
}
