mod cli_args;
mod db_interactions;
mod main_menu;
mod siv_utils;
mod test;
mod tui_config;
mod tui_logs;
mod tui_tables;

use anyhow::Result;
use cursive::{Cursive, CursiveExt};
use db_interactions::setup_db;
use main_menu::draw_main_menu;
use siv_utils::{check_config, quit};

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();

    check_config(&mut siv);
    draw_main_menu(&mut siv);

    siv.add_global_callback('q', quit);

    siv.run();
    Ok(())
}
