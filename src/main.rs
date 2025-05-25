mod cli_args;
mod db_interactions;
mod tui_tables;
mod siv_utils;

use color_eyre::Result;
use cursive::{ Cursive, CursiveExt };
use db_interactions::setup_db;
use tui_tables::draw_db_explorer;
use siv_utils::{check_config, draw_bottom_bar, info, quit, show_help};

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();
    draw_bottom_bar(&mut siv);
    check_config(&mut siv);

    siv.add_global_callback('q', quit);
    siv.add_global_callback('i', info);
    siv.add_global_callback('t', draw_db_explorer);
    siv.add_global_callback('h', show_help);

    siv.run();
    Ok(())
}

