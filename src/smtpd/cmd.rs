// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/cmd.rs
// Handle SMTP Command.

use super::{allow, session};
use crate::constants;

pub async fn global_command(
    session: &mut session::SmtpSession,
    para: &Vec<&str>,
) -> anyhow::Result<bool> {
    match para[0] {
        "HELO" => {
            if para.len() == 2 {
                session.client = para[1].to_string();
                let message = format!("250 {} {}\r\n", session.config.hello, session.config.domain);
                session.stream.get_writer().send(message).await?;
                session.status = session::SmtpSessionStatus::Hello;
                session.stream.clear().await;
            } else {
                allow::SmtpError::new(501)
                    .return_code(session.stream.get_writer())
                    .await?;
            }
        }
        "EHLO" => {
            //TODO
            return Ok(false);
        }
        "NOOP" => {
            if para.len() != 1 {
                allow::SmtpError::new(501)
                    .return_code(session.stream.get_writer())
                    .await?;
            }
            session
                .stream
                .get_writer()
                .send(String::from("250 2.0.0 OK\r\n"))
                .await?;
        }
        "QUIT" => {
            if para.len() == 1 {
                session.status = session::SmtpSessionStatus::Stop;
                session
                    .stream
                    .get_writer()
                    .send(String::from("221 2.0.0 Bye\r\n"))
                    .await?;
            } else {
                allow::SmtpError::new(501)
                    .return_code(session.stream.get_writer())
                    .await?;
            }
        }
        "RSET" => {
            if para.len() == 1 {
                session.status = session::SmtpSessionStatus::Start;
                session.stream.clear().await;
            } else {
                allow::SmtpError::new(501)
                    .return_code(session.stream.get_writer())
                    .await?;
            }
        }
        "VRFY" => {
            //TODO
            return Ok(false);
        }
        "HELP" => {
            if para.len() == 1 {
                session
                    .stream
                    .get_writer()
                    .send(String::from(constants::SMTPD_HELP))
                    .await?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
