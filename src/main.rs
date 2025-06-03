use mqttui::{db_interactions, main_menu, siv_utils};

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
