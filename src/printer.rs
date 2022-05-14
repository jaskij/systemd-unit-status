use std::collections::BTreeMap;

use anyhow::Result;
use clap::{ArgEnum, Args};

use crate::unit::UnitStatus;

#[derive(Copy, Clone, Debug, ArgEnum)]
pub(crate) enum OutpuType {
    Json,
    Table,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct PrintConfig {
    #[clap(short = 'c', long = "color")]
    /// Force colored output.
    force_color: bool,

    #[clap(short = 't', long, arg_enum)]
    /// Output type, defaults to table for TTY, JSON otherwise
    output_type: Option<OutpuType>,
}

pub(crate) fn print(data: BTreeMap<String, UnitStatus>, _config: PrintConfig) -> Result<()> {
    for (unit, status) in data {
        println!("{}: {}", unit, status);
    }

    Ok(())
}

fn print_nice_table(data: BTreeMap<String, UnitStatus>) {}
