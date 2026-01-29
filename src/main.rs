// Copyright (c) 2026 Arc Asumity
// Licensed under the GPLv3 or later License.
// See LICENSE file for details.
//
// src/main.rs
// Entry point.

mod allow;
mod conf;
mod constants;
mod manager;
mod smtpd;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_list = init("example/arcmail.json").await?;
    let server_control = run(server_list)?;
    let _ = sleep(server_control).await;
    Ok(())
}

async fn init(path: &str) -> anyhow::Result<manager::ServerList> {
    let config = conf::Config::load_path(path)?;

    let mut smtpds: Vec<smtpd::server::SmtpServer> = Vec::with_capacity(config.smtp_servers.len());
    for smtpd_config in &config.smtp_servers {
        let mut smtpd = smtpd::server::SmtpServer::new(
            smtpd_config.binds.len() * smtpd_config.ports.len(),
            smtpd_config.clone(),
        )?;
        for (bind, port) in smtpd_config.binds.iter().zip(smtpd_config.ports.iter()) {
            let port_str = port.to_string();
            let mut addr = String::with_capacity(bind.len() + port_str.len() + 1);
            addr.push_str(&bind);
            addr.push_str(":");
            addr.push_str(&port_str);
            smtpd.add(&addr).await?;
        }
        smtpds.push(smtpd);
    }
    Ok(manager::ServerList { smtpds })
}

fn run(servers: manager::ServerList) -> anyhow::Result<manager::ServerControl> {
    let smtpds_control = servers
        .smtpds
        .into_iter()
        .map(|smtpd: smtpd::server::SmtpServer| smtpd.run())
        .collect();

    Ok(manager::ServerControl { smtpds_control })
}

async fn sleep(controls: manager::ServerControl) -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl C");
        }
        _ = controls.stop() => {
            println!("Normal");
        }
    }
    Ok(())
}
