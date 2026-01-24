// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd.rs
// SMTP Server.

use crate::allow;
use crate::constants;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    Init,
    Hello,
    Ehllo,
    Sender,
    Rcpt,
    Data,
}

pub struct SmtpSession {
    domain: Arc<str>,
    addr: SocketAddr,
    stream: TcpStream,
    status: SmtpSessionStatus,
    tls: bool,
    client: String,
}

impl SmtpSession {
    pub fn new(domain: Arc<str>, addr: SocketAddr, stream: TcpStream) -> Self {
        SmtpSession {
            domain,
            addr,
            stream,
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
        let (reader, mut writer) = self.stream.split();
        let mut reader = BufReader::new(reader);
        let hello = format!(
            "220 {} {} {}\r\n",
            self.domain,
            constants::SMTPD_HELLO,
            constants::SMTPD_NAME
        );
        self.status = SmtpSessionStatus::Init;
        writer.write_all(hello.as_bytes()).await?;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            let line = match remove_crlf(line) {
                Ok(line) => line,
                Err(e) => {
                    e.return_code(&mut writer);
                    continue;
                }
            };
            if line.is_empty() {
                allow::SmtpError::new(500).return_code(&mut writer).await?;
                continue;
            }
            let line_parts: Vec<&str> = line.split_whitespace().collect();
            match self.status {
                SmtpSessionStatus::Init => match line_parts[0] {
                    "HELO" => {
                        if line_parts.len() == 2 {
                            self.client = line_parts[1].to_string();
                            self.status = SmtpSessionStatus::Hello;
                        } else {
                            allow::SmtpError::new(501).return_code(&mut writer).await?;
                            continue;
                        }
                    }
                    _ => {
                        allow::check_command(line_parts[0])
                            .return_code(&mut writer)
                            .await?;
                    }
                },
                _ => {}
            }
        }
        Ok(false)
    }
}

fn remove_crlf(line: String) -> Result<String, allow::SmtpError> {
    if !line.ends_with("\r\n") {
        return Err(allow::SmtpError::new(500));
    } else {
        Ok(line.trim_end_matches("\r\n").to_string())
    }
}
