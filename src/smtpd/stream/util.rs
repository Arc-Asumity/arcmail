// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/stream/util.rs
// Common method of of SMTPd stream.

use tokio::io::WriteHalf;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub async fn clear_writer<S>(writer: &mut S, mut rx: mpsc::Receiver<String>) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    while let Ok(mes) = rx.try_recv() {
        writer.write_all(mes.as_bytes()).await?;
    }
    Ok(())
}

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
