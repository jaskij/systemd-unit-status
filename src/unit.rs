use std::fmt::{Display, Formatter};

use anyhow::Result;

use crate::dbus::systemd;
use crate::dbus::unit;

#[derive(Clone, Copy, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
}

impl TryFrom<String> for ActiveState {
    type Error = strum::ParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

pub(crate) struct UnitStatus {
    pub(crate) state: ActiveState,
    pub(crate) sub_state: String,
}

impl Display for UnitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.state, self.sub_state)
    }
}

pub(crate) async fn get_unit_status(
    unit_name: &str,
    dbus_connection: &zbus::Connection,
    systemd_proxy: &systemd::ManagerProxy<'_>,
) -> Result<UnitStatus, zbus::Error> {
    let path = systemd_proxy.get_unit(unit_name).await?;

    let service_proxy = unit::UnitProxy::builder(dbus_connection)
        .path(path)?
        .build()
        .await?;

    Ok(UnitStatus {
        state: service_proxy
            .active_state()
            .await?
            .try_into()
            .expect("systemd sent unknown state"),
        sub_state: service_proxy.sub_state().await?,
    })
}

pub(crate) fn is_valid_unit_name(name: &str) -> bool {
    let valid_suffixes = [
        ".service",
        ".socket",
        ".device",
        ".mount",
        ".automount",
        ".swap",
        ".target",
        ".path",
        ".timer",
        ".slice",
        ".scope",
    ];

    for suffix in valid_suffixes {
        if name.ends_with(suffix) {
            return true;
        }
    }

    false
}

pub(crate) fn fix_unit_name(name: String) -> String {
    match is_valid_unit_name(&name) {
        true => name,
        false => format!("{}.service", name),
    }
}
