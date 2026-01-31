// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/stream/tls.rs
// Stream of SMTPd.

pub struct TlsSmtpStream {
    pub write_tx: mpsc::Sender<String>,
    pub read_rx: mpsc::Receiver<String>,
    control: watch::Sender<SmtpStreamControl>,
    handler: Option<task::JoinHandle<anyhow::Result<TlsStream<TcpStream>>>>,
}
