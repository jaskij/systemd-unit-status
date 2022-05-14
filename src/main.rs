use std::error::Error;

use crate::stream::FuturesUnordered;
use anyhow::Result;
use clap::Parser;
use futures::{stream, TryStreamExt};

mod dbus;
mod printer;
mod unit;

use crate::unit::{fix_unit_name, UnitStatus};

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
struct Config {
    #[clap(required(true))]
    /// Units to show status for.
    units: Vec<String>,

    #[clap(flatten)]
    print_config: printer::PrintConfig,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::parse();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(config))?;

    Ok(())
}

async fn run(config: Config) -> Result<()> {
    let connection = zbus::Connection::system().await?;
    let proxy = dbus::systemd::ManagerProxy::new(&connection).await?;

    let data = config
        .units
        .into_iter()
        .map(|n| get_status(n, &connection, &proxy))
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await?;

    printer::print(data, config.print_config)
}

async fn get_status(
    name: String,
    connection: &zbus::Connection,
    systemd_proxy: &dbus::systemd::ManagerProxy<'_>,
) -> Result<(String, UnitStatus), zbus::Error> {
    let unit_name = fix_unit_name(name);
    let status = unit::get_unit_status(&unit_name, &connection, &systemd_proxy).await?;
    Ok((unit_name, status))
}
