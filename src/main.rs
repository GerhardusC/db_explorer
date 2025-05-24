mod db_interactions;
mod tui_tables;
mod siv_utils;

use color_eyre::Result;
use cursive::{ views:: TextView , Cursive, CursiveExt };
use db_interactions::setup_db;
use tui_tables::init_table_selection;
use siv_utils::{draw_bottom_bar, info, quit, show_help};

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();
    draw_bottom_bar(&mut siv);

    siv.add_layer(TextView::new(""));
    info(&mut siv);

    siv.add_global_callback('q', quit);
    siv.add_global_callback('i', info);
    siv.add_global_callback('t', init_table_selection);
    siv.add_global_callback('h', show_help);

    siv.run();
    Ok(())
}

