use cursive::{
    Cursive,
    views::{Button, Dialog, LinearLayout},
};

use crate::{tui_logs::draw_logs, tui_tables::draw_db_explorer};

pub fn draw_main_menu(s: &mut Cursive) {
    let main_menu_id = s.add_screen();

    let tables_screen_id = s.add_screen();
    s.set_screen(tables_screen_id);
    draw_db_explorer(s, main_menu_id);

    let logs_screen_id = s.add_screen();
    s.set_screen(logs_screen_id);
    draw_logs(s, main_menu_id);

    s.add_global_callback('b', move |s| {
        s.set_screen(main_menu_id);
    });
    s.set_screen(main_menu_id);
    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(Button::new("TABLES", move |s| {
                    s.set_screen(tables_screen_id);
                }))
                .child(Button::new("LOGS", move |s| {
                    s.set_screen(logs_screen_id);
                }))
                .child(Button::new("QUIT", |s| {
                    s.quit();
                })),
        )
        .title("Main Menu"),
    );

    // This doesn't work in tmux :(
    s.menubar().add_subtree(
        "Navigation",
        cursive::menu::Tree::new()
            .leaf("Tables", move |s| {
                s.set_screen(tables_screen_id);
            })
            .leaf("Logs", move |s| {
                s.set_screen(logs_screen_id);
            })
            .leaf("Main menu", move |s| {
                s.set_screen(main_menu_id);
            }),
    );
}
