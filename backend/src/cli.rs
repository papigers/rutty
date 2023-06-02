use std::{net::IpAddr, ops::RangeInclusive};

use clap::{Args, Parser};
use serde::Serialize;
use tracing::debug;

/// RuTTY - Rust TTY Server
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Command
    pub command: String,

    /// Arguments
    pub args: Vec<String>,

    /// Listening address
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: IpAddr,

    /// Listening port
    #[arg(short, long, default_value_t = 3000, value_parser = port_in_range)]
    pub port: u16,

    /// Allow writing to the command
    #[arg(short = 'w', long)]
    pub allow_write: bool,

    #[clap(flatten, next_help_heading = "Client Options")]
    pub client_config: ClientConfig,
}

/// Client relevant configuration
#[derive(Args, Debug, Clone, Serialize)]
#[group()]
pub struct ClientConfig {
    /// HTML title
    #[arg(short, long, default_value = "RuTTY Server")]
    pub title: String,

    /// Automatic socket reconnection delay
    #[arg(short, long)]
    pub reconnect: Option<u16>,
}

const PORT_RANGE: RangeInclusive<usize> = 1..=65535;

fn port_in_range(s: &str) -> Result<u16, String> {
    let port: usize = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a port number"))?;
    if PORT_RANGE.contains(&port) {
        Ok(port as u16)
    }
    else {
        Err(format!(
            "port not in range {}-{}",
            PORT_RANGE.start(),
            PORT_RANGE.end()
        ))
    }
}

pub fn parse() -> Config {
    let config = Config::parse();
    debug!("Parsed configuration: {config:#?}");
    config
}
