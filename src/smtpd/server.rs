// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/server.rs
// Server of SMTPd.

use super::{cmd, session};
use crate::{conf, constants};
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
                                    let session: session::SmtpSession = session::SmtpSession::new(config.clone(), addr, stream);
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
