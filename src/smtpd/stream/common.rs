// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/stream/common.rs
// Common method of of SMTPd stream.

use super::{tcp, util};
use std::marker::{Send, Unpin};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task;
use tokio_rustls::server::TlsStream;

pub enum SmtpStream {
    Tcp(tcp::TcpSmtpStream),
    //Tls(TlsSmtpStream),
}

impl SmtpStream {
    pub fn new(stream: TcpStream, write_size: usize, read_size: usize) -> Self {
        let (handler, read_rx, write_tx, control) = new::<TcpStream>(stream, write_size, read_size);
        SmtpStream::Tcp(tcp::TcpSmtpStream {
            write_tx,
            read_rx,
            control,
            handler: Some(handler),
        })
    }

    //pub fn starttls() -> Self {}
}

impl SmtpStreamTrait for SmtpStream {
    async fn stop(&mut self) -> anyhow::Result<()> {
        match self {
            SmtpStream::Tcp(stream) => stream.stop().await,
        }
    }

    fn clear(&mut self) {
        match self {
            SmtpStream::Tcp(stream) => stream.clear(),
        }
    }

    async fn read_line(&mut self) -> Option<String> {
        match self {
            SmtpStream::Tcp(stream) => stream.read_line().await,
        }
    }

    fn get_writer(&mut self) -> mpsc::Sender<String> {
        match self {
            SmtpStream::Tcp(stream) => stream.write_tx.clone(),
        }
    }
}

pub trait SmtpStreamTrait {
    async fn stop(&mut self) -> anyhow::Result<()>;
    fn clear(&mut self);
    async fn read_line(&mut self) -> Option<String>;
    fn get_writer(&mut self) -> mpsc::Sender<String>;
}

pub enum SmtpStreamControl {
    Initialize,
    ShutdownReady,
    Shutdown,
}

pub fn new<S>(
    stream: S,
    write_size: usize,
    read_size: usize,
) -> (
    task::JoinHandle<anyhow::Result<S>>,
    mpsc::Receiver<String>,
    mpsc::Sender<String>,
    watch::Sender<SmtpStreamControl>,
)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (write_tx, mut write_rx) = mpsc::channel::<String>(write_size);
    let (read_tx, read_rx) = mpsc::channel::<String>(read_size);
    let (control, mut answer) = watch::channel(SmtpStreamControl::Initialize);
    let handler: task::JoinHandle<anyhow::Result<S>> =
        tokio::spawn(async move { create::<S>(stream, write_rx, read_tx, answer).await });
    (handler, read_rx, write_tx, control)
}

pub async fn create<S>(
    stream: S,
    mut write_rx: mpsc::Receiver<String>,
    read_tx: mpsc::Sender<String>,
    mut answer: watch::Receiver<SmtpStreamControl>,
) -> anyhow::Result<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(stream);
    let mut line = Vec::<u8>::new();
    loop {
        tokio::select! {
            ctrl = answer.changed() => {
                match ctrl {
                    Initialize => {
                        continue;
                    }
                    ShutdownReady => {
                        match answer.changed() {
                            Shutdown => {
                                util::clear_writer(reader.get_mut(), write_rx).await?;
                                return Ok(reader.into_inner());
                            }
                            _ => continue,
                        }
                    }
                    Shutdown => {
                        util::clear_writer(reader.get_mut(), write_rx).await?;
                        return Ok(reader.into_inner());
                    }
                }
            }
            message = write_rx.recv() => {
                let mes = match message {
                    Some(s) => s,
                    None => return Ok(reader.into_inner()),
                };
                reader.get_mut().write_all(mes.as_bytes()).await.map_err(anyhow::Error::from)?;
            }
            res = util::read_line_limit(&mut reader, &mut line, false, 512) => {
                match res {
                    Ok(len) => {
                        if len == 0 {
                            return Ok(reader.into_inner());
                        }
                        match String::from_utf8(line) {
                            Ok(mes) => read_tx.send(mes).await.map_err(anyhow::Error::from)?,
                            Err(_) => reader.get_mut().write_all(b"501 5.5.2 Unrecognized code").await.map_err(anyhow::Error::from)?,
                        };
                    }
                    Err(e) => {
                        match e {
                            util::UtilReadError::TooLong => reader.get_mut().write_all(b"501 5.5.2 The line is too long").await.map_err(anyhow::Error::from)?,
                            util::UtilReadError::NotAscii => reader.get_mut().write_all(b"501 5.5.2 Not ASCII").await.map_err(anyhow::Error::from)?,
                            util::UtilReadError::NetError(e) => return Err(e.into()),
                        };
                    }
                };
                line = Vec::<u8>::new();
            }
        }
    }
}
