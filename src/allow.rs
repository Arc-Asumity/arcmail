// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/allow.rs
// Check Command.

use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedWriteHalf, WriteHalf};

pub enum SmtpError {
    SyntaxError(&'static str),
    ParamSyntaxError(&'static str),
    CommandNotImplemented(&'static str),
    BadSequence(&'static str),
}

impl SmtpError {
    pub fn new(code: u16) -> Self {
        match code {
            500 => SmtpError::SyntaxError("500 Syntax error, command unrecognized\r\n"),
            501 => SmtpError::ParamSyntaxError("501 Syntax error in parameters or arguments\r\n"),
            502 => SmtpError::CommandNotImplemented("502 Command not implemented\r\n"),
            503 => SmtpError::BadSequence("503 Bad sequence of commands\r\n"),
            _ => SmtpError::SyntaxError("500 Syntax error, command unrecognized\r\n"),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            SmtpError::SyntaxError(msg) => msg,
            SmtpError::ParamSyntaxError(msg) => msg,
            SmtpError::CommandNotImplemented(msg) => msg,
            SmtpError::BadSequence(msg) => msg,
        }
    }

    pub async fn return_code(self, writer: &mut WriteHalf<'_>) -> anyhow::Result<()> {
        writer.write_all(self.message().as_bytes()).await?;
        Ok(())
    }
}

const RFC5321_COMMANDS: [&str; 11] = [
    "HELO", "EHLO", "MAIL", "RCPT", "DATA", "RSET", "NOOP", "QUIT", "VRFY", "EXPN", "HELP",
];

const ALLOW_COMMANDS: [&str; 1] = ["HELO"];

#[inline]
pub fn check_command(command: &str) -> SmtpError {
    if !RFC5321_COMMANDS.contains(&command) {
        SmtpError::new(500)
    } else if ALLOW_COMMANDS.contains(&command) {
        SmtpError::new(503)
    } else {
        SmtpError::new(502)
    }
}
