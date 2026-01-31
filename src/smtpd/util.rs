// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/util.rs
// Tools of SMTPd.

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

pub async fn write_multi_response(
    writer: mpsc::Sender<String>,
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
        writer.send(line).await?;
    }
    Ok(())
}

pub fn remove_crlf(line: &mut String) {
    let len = line
        .char_indices()
        .rev()
        .nth(1)
        .map(|(i, _)| i)
        .unwrap_or(0);
    line.truncate(len);
}
