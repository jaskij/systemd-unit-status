use std::fmt::{Display, Formatter};
use std::time::Duration;

use anyhow::Result;

use crate::dbus::systemd;
use crate::dbus::unit::UnitProxy;

#[derive(Clone, Copy, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ActiveState {
    Active,
    Activating,
    Deactivating,
    Failed,
    Inactive,
    Reloading,
}

impl TryFrom<String> for ActiveState {
    type Error = strum::ParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

pub(crate) struct UnitState {
    pub(crate) state: ActiveState,
    pub(crate) sub_state: String,
}

impl Display for UnitState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.state, self.sub_state)
    }
}

impl UnitState {
    pub(crate) fn to_cell(&self) -> comfy_table::Cell {
        use comfy_table::{Cell, Color};

        let cell = Cell::new(self.to_string());
        match self.state {
            ActiveState::Active => cell.fg(Color::Green),
            ActiveState::Failed => cell.fg(Color::Red),
            _ => cell,
        }
    }
}

pub(crate) struct UnitInfo {
    pub(crate) state: UnitState,
    pub(crate) time_since_state_change: Duration,
}

pub(crate) async fn get_info(
    unit_name: &str,
    dbus_connection: &zbus::Connection,
    systemd_proxy: &systemd::ManagerProxy<'_>,
    current_clock_monotonic_usec: u64,
) -> Result<UnitInfo, zbus::Error> {
    let path = systemd_proxy.get_unit(unit_name).await?;

    let unit_proxy = UnitProxy::builder(dbus_connection)
        .path(path)?
        .build()
        .await?;

    let state = unit_proxy
        .active_state()
        .await?
        .try_into()
        .expect("systemd sent unknown state");

    Ok(UnitInfo {
        state: UnitState {
            state,
            sub_state: unit_proxy.sub_state().await?,
        },
        time_since_state_change: get_time_since_transition(
            state,
            &unit_proxy,
            current_clock_monotonic_usec,
        )
        .await?,
    })
}

async fn get_time_since_transition(
    state: ActiveState,
    proxy: &UnitProxy<'_>,
    current_clock_monotonic_usec: u64,
) -> Result<Duration, zbus::Error> {
    let transition_usec = match state {
        ActiveState::Active => proxy.active_enter_timestamp_monotonic().await?,
        ActiveState::Activating => proxy.inactive_exit_timestamp_monotonic().await?,
        ActiveState::Deactivating => proxy.active_exit_timestamp_monotonic().await?,
        ActiveState::Failed => proxy.inactive_enter_timestamp_monotonic().await?,
        ActiveState::Inactive => proxy.inactive_enter_timestamp_monotonic().await?,
        ActiveState::Reloading => proxy.active_enter_timestamp_monotonic().await?,
    };
    Ok(Duration::from_secs(
        (current_clock_monotonic_usec - transition_usec) / 1_000_000,
    ))
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
