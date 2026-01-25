// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/esmtpd.rs
// Expand SMTP Server.

use crate::allow;
use crate::smtpd;
use crate::smtpd_cmd;
use tokio::net::tcp::OwnedWriteHalf;

pub enum EsmtpStatus {
    Empty,
}

const ALLOW_EXPEND_MESSAGE: [&str; 1] = [""];

pub async fn run(session: &mut smtpd::SmtpSession) -> anyhow::Result<()> {
    let mut messages = ALLOW_EXPEND_MESSAGE.to_vec();
    messages[0] = &session.config.domain;
    smtpd_cmd::write_multi_response(&mut session.writer, messages).await?;
    session.status = smtpd::SmtpSessionStatus::Hello;
    Ok(())
}
