mod cli_args;
mod db_interactions;
mod siv_utils;
mod test;
mod tui_logs;
mod tui_tables;

use color_eyre::Result;
use cursive::{Cursive, CursiveExt};
use db_interactions::setup_db;
use siv_utils::{check_config, draw_bottom_bar, info, quit, show_help};
use tui_logs::draw_logs;
use tui_tables::draw_db_explorer;

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();
    draw_bottom_bar(&mut siv);
    check_config(&mut siv);
    // draw_startup_popup(&mut siv);


    siv.add_global_callback('q', quit);
    siv.add_global_callback('i', info);
    siv.add_global_callback('t', draw_db_explorer);
    siv.add_global_callback('l', draw_logs);
    siv.add_global_callback('h', show_help);

    siv.run();
    Ok(())
}
