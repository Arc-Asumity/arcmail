// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/stream.rs
// Stream of SMTPd.

use super::util;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task;
use tokio_rustls::server::TlsStream;

pub enum SmtpStream {
    Tcp(TcpSmtpStream),
    Tls(TlsSmtpStream),
}

impl SmtpStream {
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        match self {
            SmtpStream::Tcp(stream) => stream.stop().await,
            SmtpStream::Tls(stream) => {
                //TODO
                Ok(())
            }
        }
    }

    pub async fn clear(&mut self) {
        match self {
            SmtpStream::Tcp(stream) => stream.clear(),
            SmtpStream::Tls(stream) => {
                //TODO
            }
        }
    }

    pub async fn read_line(&mut self) -> Option<String> {
        match self {
            SmtpStream::Tcp(stream) => stream.read_line().await,
            SmtpStream::Tls(stream) => {
                //TODO
                Some(String::new())
            }
        }
    }

    pub fn get_writer(&mut self) -> mpsc::Sender<String> {
        match self {
            SmtpStream::Tcp(stream) => stream.write_tx.clone(),
            SmtpStream::Tls(stream) => stream.write_tx.clone(),
        }
    }
}

pub enum SmtpStreamControl {
    Initialize,
    Shutdown,
}

pub struct TcpSmtpStream {
    pub write_tx: mpsc::Sender<String>,
    pub read_rx: mpsc::Receiver<String>,
    control: watch::Sender<SmtpStreamControl>,
    handler: Option<task::JoinHandle<anyhow::Result<TcpStream>>>,
}

pub struct TlsSmtpStream {
    pub write_tx: mpsc::Sender<String>,
    pub read_rx: mpsc::Receiver<String>,
    control: watch::Sender<SmtpStreamControl>,
    handler: Option<task::JoinHandle<anyhow::Result<TlsStream<TcpStream>>>>,
}

impl TcpSmtpStream {
    pub fn new(stream: TcpStream, write_size: usize, read_size: usize) -> Self {
        let (write_tx, mut write_rx) = mpsc::channel::<String>(write_size);
        let (read_tx, read_rx) = mpsc::channel::<String>(read_size);
        let (control, mut answer) = watch::channel(SmtpStreamControl::Initialize);
        let handler: task::JoinHandle<anyhow::Result<TcpStream>> = tokio::spawn(async move {
            let mut stm = stream;
            let (reader, mut writer) = stm.split();
            let mut reader = BufReader::new(reader);
            let mut line = Vec::<u8>::new();
            loop {
                tokio::select! {
                    ctrl = answer.changed() => {
                        match ctrl {
                            Initialize => {
                                continue;
                            }
                            Shutdown => {
                                return Ok(stm);
                            }
                        }
                    }
                    message = write_rx.recv() => {
                        let mes = match message {
                            Some(s) => s,
                            None => return Ok(stm),
                        };
                        writer.write_all(mes.as_bytes()).await.map_err(anyhow::Error::from)?;
                    }
                    res = util::read_line_limit(&mut reader, &mut line, false, 512) => {
                        match res {
                            Ok(len) => {
                                if len == 0 {
                                    return Ok(stm);
                                }
                                match String::from_utf8(line) {
                                    Ok(mes) => read_tx.send(mes).await.map_err(anyhow::Error::from)?,
                                    Err(_) => writer.write_all(b"501 5.5.2 Unrecognized code").await.map_err(anyhow::Error::from)?,
                                };
                            }
                            Err(e) => {
                                match e {
                                    util::UtilReadError::TooLong => writer.write_all(b"501 5.5.2 The line is too long").await.map_err(anyhow::Error::from)?,
                                    util::UtilReadError::NotAscii => writer.write_all(b"501 5.5.2 Not ASCII").await.map_err(anyhow::Error::from)?,
                                    util::UtilReadError::NetError(e) => return Err(e.into()),
                                };
                            }
                        };
                        line = Vec::<u8>::new();
                    }
                }
            }
        });
        TcpSmtpStream {
            write_tx,
            read_rx,
            control,
            handler: Some(handler),
        }
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        let _ = self.control.send(SmtpStreamControl::Shutdown);
        let mut stream = match self.handler.take() {
            Some(handler) => handler.await?,
            None => return Ok(()),
        };
        let _ = stream?.shutdown().await;
        Ok(())
    }

    pub fn clear(&mut self) {
        while let Ok(_mes) = self.read_rx.try_recv() {}
    }

    pub async fn read_line(&mut self) -> Option<String> {
        self.read_rx.recv().await
    }
}
