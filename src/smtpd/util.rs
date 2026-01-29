// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/util.rs
// Tools of SMTPd.

use crate::allow;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncRead;
use tokio::io::BufReader;
use tokio::net::tcp::WriteHalf;
use tokio::sync::mpsc;

pub enum UtilReadError {
    TooLong,
    NotAscii,
    NetError(std::io::Error),
}

pub async fn read_line_limit<R>(
    reader: &mut BufReader<R>,
    data: &mut Vec<u8>,
    eight_bit: bool,
    limit: usize,
) -> Result<usize, UtilReadError>
where
    R: AsyncRead + Unpin,
{
    if !eight_bit {
        let mut can_save = true;
        loop {
            let buf: std::io::Result<&[u8]> = reader.fill_buf().await;
            let buf = match buf {
                Ok(mes) => mes,
                Err(e) => return Err(UtilReadError::NetError(e)),
            };
            if buf.is_empty() {
                return Ok(0);
            }
            if let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
                let len = pos + 2;
                if add_limit(data, &buf[..len], limit, can_save) && check_8bit(data) {
                    reader.consume(len);
                    return Ok(data.len());
                } else {
                    reader.consume(len);
                    return Err(UtilReadError::NotAscii);
                }
            }
            can_save = add_limit(data, buf, limit, can_save);
        }
    } else {
        return Err(UtilReadError::TooLong);
    }
}

fn add_limit(list: &mut Vec<u8>, buf: &[u8], limit: usize, can_save: bool) -> bool {
    if check_limit(list, buf.len(), limit) || !can_save {
        list.clear();
        return false;
    } else {
        list.extend_from_slice(buf);
        return true;
    }
}

fn check_limit(list: &Vec<u8>, len: usize, limit: usize) -> bool {
    list.len() + len > limit
}

fn check_8bit(data: &Vec<u8>) -> bool {
    data.iter().all(|&b| b.is_ascii())
}

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
