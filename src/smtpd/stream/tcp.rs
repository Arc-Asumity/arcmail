// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/stream/tcp.rs
// Stream of SMTPd.

use super::common;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task;

pub struct TcpSmtpStream {
    pub write_tx: mpsc::Sender<String>,
    pub read_rx: mpsc::Receiver<String>,
    pub control: watch::Sender<common::SmtpStreamControl>,
    pub handler: Option<task::JoinHandle<anyhow::Result<TcpStream>>>,
}

impl common::SmtpStreamTrait for TcpSmtpStream {
    async fn stop(&mut self) -> anyhow::Result<()> {
        let _ = self.control.send(common::SmtpStreamControl::Shutdown);
        let stream = match self.handler.take() {
            Some(handler) => handler.await?,
            None => return Ok(()),
        };
        let _ = stream?.shutdown().await;
        Ok(())
    }

    fn clear(&mut self) {
        while let Ok(_mes) = self.read_rx.try_recv() {}
    }

    async fn read_line(&mut self) -> Option<String> {
        self.read_rx.recv().await
    }

    fn get_writer(&mut self) -> mpsc::Sender<String> {
        self.write_tx.clone()
    }
}
