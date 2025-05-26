use std::path::Path;

use cursive::{
    Cursive, Rect, Vec2, View, XY,
    view::{Nameable, Resizable},
    views::{Dialog, FixedLayout, Layer, NamedView, OnLayoutView, TextView},
};

use crate::{cli_args::ARGS, tui_tables::draw_db_explorer};

pub fn check_config(s: &mut Cursive) {
    s.add_layer(TextView::new(""));
    if let Err(_) = s.load_toml(include_str!("./config.toml")) {
        let mut view = Dialog::info("No config file found, using default styles");
        view.buttons_mut().for_each(|i| {
            i.set_callback(|s| {
                draw_db_explorer(s);
            });
        });

        s.add_layer(view);
    } else {
        draw_db_explorer(s);
    }
}

pub fn quit(s: &mut Cursive) {
    s.quit()
}

pub fn info(s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("general_info", |_v: &mut Dialog| {}) {
        s.pop_layer();
    };
    s.add_layer(
        Dialog::text("-> <t> for tables")
            .title("INSTRUCTIONS:")
            .with_name("general_info"),
    );
}

pub fn draw_startup_popup(s: &mut Cursive) {
    let path = Path::new(&ARGS.db_path);
    if let Ok(path) = path.canonicalize() {
        if let Some(path) = path.to_str() {
            s.add_layer(
                Dialog::info(format!("Starting with DB at:\n{}", path)).title("Startup info"),
            );
        };
    } else {
        s.add_layer(
            Dialog::info(format!("Failed to parse DB Path:\n{}", &ARGS.db_path))
                .title("Startup info"),
        );
    };
}

pub fn show_help(s: &mut Cursive) {
    if let Some(_) = s.call_on_name("help_menu", |_v: &mut NamedView<Dialog>| {}) {
        s.pop_layer();
    };

    s.add_layer(
        Dialog::info("TODO! Write info")
            .title("HELP")
            .with_name("help_menu"),
    );
}

pub fn draw_bottom_bar(s: &mut Cursive) {
    s.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                Layer::new(TextView::new("<T>ables | <I>nfo | <H>elp | <Q>uit ")).full_width(),
            ),
            draw_bottom_bar_cb,
        )
        .full_screen()
        .with_name("bottom_bar"),
    );

    s.add_layer(TextView::new(""));
}

fn draw_bottom_bar_cb(layout: &mut FixedLayout, size: XY<usize>) {
    let rect = cursive::Rect::from_size((0, size.y - 1), (size.x, 1));
    layout.set_child_position(0, rect);
    layout.layout(size);
}
