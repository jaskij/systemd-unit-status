use std::error::Error;

use anyhow::Result;
use clap::{Args, Parser};
use futures::{stream::FuturesUnordered, TryStreamExt};

mod dbus;
mod printer;
mod unit;

use crate::unit::UnitInfo;

#[derive(Args, Clone, Debug)]
struct UnitListConfig {
    #[clap(required(true))]
    /// Units to show status for
    units: Vec<String>,
}

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
struct Config {
    #[clap(flatten)]
    unit_list_config: UnitListConfig,

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
    let dbus_connection = zbus::Connection::system().await?;
    let systemd_proxy = dbus::systemd::ManagerProxy::new(&dbus_connection).await?;

    let current_clock_monotonic_usec = get_clock_monotonic_usec();

    let data = prep_unit_list(config.unit_list_config, &systemd_proxy)
        .await?
        .into_iter()
        .map(|(unit_name, unit_path)| {
            get_unit_info(
                unit_name,
                unit_path,
                &dbus_connection,
                current_clock_monotonic_usec,
            )
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await?;

    printer::print(data, config.print_config)
}

async fn prep_unit_list(
    config: UnitListConfig,
    systemd_proxy: &dbus::systemd::ManagerProxy<'_>,
) -> Result<Vec<(String, zbus::zvariant::OwnedObjectPath)>, zbus::Error> {
    Ok(config
        .units
        .into_iter()
        .map(unit::fix_unit_name)
        .map(|unit_name| get_unit_path(unit_name, systemd_proxy))
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await?)
}

async fn get_unit_info(
    unit_name: String,
    unit_path: zbus::zvariant::OwnedObjectPath,
    dbus_connection: &zbus::Connection,
    current_clock_monotonic_usec: u64,
) -> zbus::Result<(String, UnitInfo)> {
    Ok((
        unit_name,
        unit::get_info(unit_path, dbus_connection, current_clock_monotonic_usec).await?,
    ))
}

async fn get_unit_path(
    unit_name: String,
    systemd_proxy: &dbus::systemd::ManagerProxy<'_>,
) -> zbus::Result<(String, zbus::zvariant::OwnedObjectPath)> {
    let path = unit::get_path(&unit_name, systemd_proxy).await?;
    Ok((unit_name, path))
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
