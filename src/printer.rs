use std::collections::BTreeMap;

use anyhow::Result;
use clap::{ArgEnum, Args};
use comfy_table::{presets, Attribute, Cell, CellAlignment, Color, Table};

use crate::unit::UnitInfo;

#[derive(Copy, Clone, Debug, ArgEnum)]
pub(crate) enum OutpuType {
    Json,
    Table,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct PrintConfig {
    #[clap(short = 'c', long = "color")]
    /// Force colored output
    force_color: bool,

    #[clap(short = 't', long, arg_enum)]
    /// Output type
    ///
    /// Defaults to table for TTY, JSON otherwise
    output_type: Option<OutpuType>,
}

pub(crate) fn print(data: BTreeMap<String, UnitInfo>, _config: PrintConfig) -> Result<()> {
    print_nice_table(data);

    Ok(())
}

fn print_nice_table(data: BTreeMap<String, UnitInfo>) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL).set_header(vec![
        header_cell("unit"),
        header_cell("status"),
        header_cell("time since\nstate transition"),
    ]);
    table
        .get_column_mut(2)
        .unwrap()
        .set_cell_alignment(CellAlignment::Right);

    for (unit, info) in data {
        table.add_row(vec![
            Cell::new(unit),
            info.state.to_cell(),
            Cell::new(humantime::format_duration(info.time_since_state_change)),
        ]);
    }

    println!("{table}");
}

fn header_cell(text: &str) -> Cell {
    Cell::new(text)
        .fg(Color::Blue)
        .add_attribute(Attribute::Bold)
}
