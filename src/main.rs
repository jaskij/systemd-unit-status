use std::error::Error;

use crate::stream::FuturesUnordered;
use anyhow::Result;
use clap::Parser;
use futures::{stream, TryStreamExt};

mod dbus;
mod printer;
mod unit;

use crate::unit::{fix_unit_name, UnitInfo};

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

    let clock_usec = get_clock_monotonic_usec();

    let data = config
        .units
        .into_iter()
        .map(|n| get_unit_info(n, &connection, &proxy, clock_usec))
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await?;

    printer::print(data, config.print_config)
}

async fn get_unit_info(
    name: String,
    connection: &zbus::Connection,
    systemd_proxy: &dbus::systemd::ManagerProxy<'_>,
    current_clock_monotonic_usec: u64,
) -> Result<(String, UnitInfo), zbus::Error> {
    let unit_name = fix_unit_name(name);
    let status = unit::get_info(
        &unit_name,
        connection,
        systemd_proxy,
        current_clock_monotonic_usec,
    )
    .await?;
    Ok((unit_name, status))
}

fn get_clock_monotonic_usec() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let ret = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) };
    assert_eq!(ret, 0, "getting clock failed unexpectedly");

    time.tv_sec as u64 * 1_000_000 + time.tv_nsec as u64 / 1_000
}
