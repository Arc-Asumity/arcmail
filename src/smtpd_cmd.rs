// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd_cmd.rs
// Handle SMTP Command.

use crate::{allow, conf, constants, esmtpd, smtpd};
use std::collections::VecDeque;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, WriteHalf};
use tokio::sync::mpsc;

pub enum LineReader {
    Stream(StreamReader),
    Buffer(BufferReader),
}

impl LineReader {
    pub async fn read_line(&mut self) -> anyhow::Result<(&str, usize)> {
        match self {
            LineReader::Stream(reader) => reader.read_line().await,
            LineReader::Buffer(reader) => reader.read_line().await,
        }
    }
}

pub struct StreamReader {
    reader: BufReader<OwnedReadHalf>,
    line: String,
}

pub struct BufferReader {
    rx: mpsc::Receiver<String>,
    line: String,
}

impl StreamReader {
    pub fn new(reader: BufReader<OwnedReadHalf>) -> Self {
        StreamReader {
            reader,
            line: String::new(),
        }
    }

    async fn read_line(&mut self) -> anyhow::Result<(&str, usize)> {
        self.line.clear();
        let len = self.reader.read_line(&mut self.line).await?;
        Ok((&self.line, len))
    }
}

impl BufferReader {
    pub async fn new(mut reader: StreamReader, channel_size: usize) -> Self {
        let (tx, rx) = mpsc::channel::<String>(channel_size);
        tokio::spawn(async move {
            loop {
                let (line, len) = reader.read_line().await.unwrap();
                if len == 0 {
                    break;
                }
                if tx.send(line.to_string()).await.is_err() {
                    break;
                }
            }
        });
        BufferReader {
            rx,
            line: String::new(),
        }
    }

    async fn read_line(&mut self) -> anyhow::Result<(&str, usize)> {
        match self.rx.recv().await {
            Some(line) => {
                self.line = line;
                Ok((&self.line, 1))
            }
            None => Ok(("", 0)),
        }
    }
}

pub async fn run(session: &mut smtpd::SmtpSession) -> anyhow::Result<bool> {
    loop {
        let (line, len) = session.reader.read_line().await?;
        if len == 0 {
            break;
        }
        let line = match remove_crlf(line) {
            Ok(line) => line,
            Err(e) => {
                e.return_code(&mut session.writer);
                continue;
            }
        };
        if line.is_empty() {
            allow::SmtpError::new(500)
                .return_code(&mut session.writer)
                .await?;
            continue;
        }
        let line_parts: Vec<&str> = line.split_whitespace().collect();
        match session.status {
            smtpd::SmtpSessionStatus::Init => match line_parts[0] {
                "HELO" => {
                    if line_parts.len() == 2 {
                        session.client = line_parts[1].to_string();
                        let message =
                            format!("250 {} {}\r\n", session.config.hello, session.config.domain);
                        session.writer.write_all(message.as_bytes()).await?;
                        session.status = smtpd::SmtpSessionStatus::Hello;
                    } else {
                        allow::SmtpError::new(501)
                            .return_code(&mut session.writer)
                            .await?;
                    }
                }
                "EHLO" => {
                    if line_parts.len() == 2 {
                        session.client = line_parts[0].to_string();
                        esmtpd::run(session).await?;
                    } else {
                        allow::SmtpError::new(501)
                            .return_code(&mut session.writer)
                            .await?;
                    }
                }
                _ => {
                    allow::check_command(line_parts[0])
                        .return_code(&mut session.writer)
                        .await?;
                }
            },
            _ => {}
        }
    }
    Ok(false)
}

pub fn remove_crlf(line: &str) -> Result<String, allow::SmtpError> {
    if !line.ends_with("\r\n") {
        return Err(allow::SmtpError::new(500));
    } else {
        Ok(line.trim_end_matches("\r\n").to_string())
    }
}

pub async fn write_multi_response(
    writer: &mut OwnedWriteHalf,
    lines: Vec<&str>,
) -> anyhow::Result<()> {
    let mut lines_string: Vec<String> = lines
        .into_iter()
        .map(|s| format!("250-{}\r\n", s))
        .collect();
    if let Some(last) = lines_string.last_mut() {
        last.replace_range(3..4, " ");
    }
    for line in lines_string {
        writer.write_all(line.as_bytes()).await?;
    }
    Ok(())
}
