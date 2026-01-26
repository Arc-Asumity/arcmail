// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd.rs
// SMTP Server.

use crate::{conf, constants, smtpd_cmd};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::{
    net::{TcpListener, TcpStream, tcp::OwnedWriteHalf},
    sync::watch,
};

#[derive(Debug)]
pub enum SmtpServerControl {
    Initialize,
    Shutdown,
}

#[derive(Debug)]
pub struct SmtpServer {
    listeners: Vec<TcpListener>,
    config: Arc<conf::ConfigSmtpServer>,
    control: watch::Sender<SmtpServerControl>,
}

impl SmtpServer {
    pub fn new(len: usize, config: Arc<conf::ConfigSmtpServer>) -> anyhow::Result<Self> {
        let listeners = Vec::with_capacity(len);
        let (control, _) = watch::channel(SmtpServerControl::Initialize);
        Ok(Self {
            listeners,
            control,
            config,
        })
    }

    pub async fn add(&mut self, addr: &str) -> anyhow::Result<()> {
        self.listeners.push(TcpListener::bind(addr).await?);
        Ok(())
    }

    pub fn run(self) -> watch::Sender<SmtpServerControl> {
        for listener in self.listeners {
            let mut rx = self.control.subscribe();
            let config = self.config.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        control = rx.changed() => {
                            match control {
                                _Initialize => {}
                                _Shutdown => {
                                    break;
                                }
                            }
                        }
                        res = listener.accept() => {
                            match res {
                                Ok((stream, addr)) => {
                                    let session: SmtpSession = SmtpSession::new(config.clone(), addr, stream);
                                    tokio::spawn( async move {
                                        session.run().await;
                                    });
                                }
                                Err(_e) => {
                                    //TODO ERR
                                }
                            }
                        }
                    }
                }
            });
        }
        self.control
    }
}

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
    pub reader: smtpd_cmd::LineReader,
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
            reader: smtpd_cmd::LineReader::Stream(smtpd_cmd::StreamReader::new(BufReader::new(
                reader,
            ))),
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
        smtpd_cmd::run(self).await
    }
}
