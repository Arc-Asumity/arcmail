// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/smtpd/esmtpd.rs
// Expand SMTP Server.

use super::{session, util};
use crate::smtpd::stream::common::SmtpStreamTrait;

pub enum EsmtpStatus {
    Empty,
}

const ALLOW_EXPEND_MESSAGE: [&str; 1] = [""];

pub async fn run(session: &mut session::SmtpSession) -> anyhow::Result<()> {
    let mut messages = ALLOW_EXPEND_MESSAGE.to_vec();
    messages[0] = &session.config.domain;
    util::write_multi_response(session.stream.get_writer(), messages).await?;
    session.status = session::SmtpSessionStatus::Hello;
    Ok(())
}
