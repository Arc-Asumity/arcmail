// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd.rs
// SMTP Server.

use crate::constants;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::{
    net::{TcpListener, TcpStream},
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
    domain: Arc<str>,
    control: watch::Sender<SmtpServerControl>,
}

impl SmtpServer {
    pub fn new(len: usize, domain: Arc<str>) -> anyhow::Result<Self> {
        let listeners = Vec::with_capacity(len);
        let (control, _) = watch::channel(SmtpServerControl::Initialize);
        Ok(Self {
            listeners,
            control,
            domain,
        })
    }

    pub async fn add(&mut self, addr: &str) -> anyhow::Result<()> {
        self.listeners.push(TcpListener::bind(addr).await?);
        Ok(())
    }

    pub fn run(self) -> watch::Sender<SmtpServerControl> {
        for listener in self.listeners {
            let mut rx = self.control.subscribe();
            let domain = self.domain.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        control = rx.changed() => {
                            match control {
                                Initialize => {}
                                Shutdown => {
                                    break;
                                }
                            }
                        }
                        res = listener.accept() => {
                            match res {
                                Ok((stream, addr)) => {
                                    let session = SmtpSession::new(domain.clone(), addr, stream);
                                    tokio::spawn( async move {
                                        session.run().await;
                                    });
                                }
                                Err(e) => {
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
    CONNECTED,
    HELLO,
    EHLLO,
    AUTH,
    DATA,
}

pub struct SmtpSession {
    domain: Arc<str>,
    addr: SocketAddr,
    stream: TcpStream,
    status: SmtpSessionStatus,
    tls: bool,
}

impl SmtpSession {
    pub fn new(domain: Arc<str>, addr: SocketAddr, stream: TcpStream) -> Self {
        SmtpSession {
            domain,
            addr,
            stream,
            status: SmtpSessionStatus::CONNECTED,
            tls: false,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.init().await;
        Ok(())
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        let hello = format!(
            "220 {} {} {}",
            self.domain,
            constants::SMTPD_HELLO,
            constants::SMTPD_NAME
        );
        self.stream.write_all(hello.as_bytes()).await?;
        Ok(())
    }
}
