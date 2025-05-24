use std::io::{Error, ErrorKind};

use color_eyre::Result;
use rusqlite::Connection;
use cursive::{
    view::Resizable,
    views::{
        Button,
        Dialog,
        FixedLayout,
        Layer,
        LinearLayout,
        OnLayoutView,
        TextView
    },
    Cursive,
    CursiveExt,
    Rect,
    Vec2,
    View, XY
};

static DB_PATH: &str = "./data.db";

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();
    draw_bottom_bar(&mut siv);
    info(&mut siv);

    siv.add_global_callback('q', quit);
    siv.add_global_callback('i', info);
    siv.add_global_callback('t', show_tables);

    siv.run();
    Ok(())
}

fn _show_encouragement (s: &mut Cursive, val: usize) {
    s.add_layer(Dialog::info(format!("Value: {}", val))
        .title("Taaitel:"));
}

fn quit (s: &mut Cursive) {
    s.quit()
}

fn info (s: &mut Cursive) {
    s.pop_layer();
    s.add_layer(Dialog::text("-> <t> for tables").title("INSTRUCTIONS:"));
}

fn show_tables (s: &mut Cursive) {
    s.pop_layer();
    let tables = get_tables().unwrap_or(Vec::new());

    let mut list = LinearLayout::vertical();
    tables.iter().for_each(|table_name| {
        list.add_child(Button::new(table_name, |_s| {}));
    });


    s.add_layer(Dialog::around(list));
}

fn draw_bottom_bar (s: &mut Cursive) {
    s.screen_mut()
        .add_transparent_layer(
            OnLayoutView::new(
                FixedLayout::new()
                    .child(
                        Rect::from_point(Vec2::zero()),
                        Layer::new(
                            TextView::new("<T>ables | <I>nfo | <Q>uit ")
                    ).full_width()
                    ),
                draw_bottom_bar_cb
            ).full_screen()
        );

    s.add_layer(TextView::new(""));
}

fn draw_bottom_bar_cb (layout: &mut FixedLayout, size: XY<usize>) {
    let rect = cursive::Rect::from_size((0, size.y - 1), (size.x, 1));
    layout.set_child_position(0, rect);
    layout.layout(size);
}

fn get_tables () -> Result<Vec<String>> {
    let conn = Connection::open(DB_PATH)?;
    let mut statement = conn.prepare("SELECT name FROM sqlite_master WHERE type='table';")?;
    let tables_iter = statement
        .query_map([], |row| { row.get::<usize, String>(0) })?;

    let tables: Result<Vec<String>, rusqlite::Error> = tables_iter.into_iter().collect();

    if let Ok(tables_vec) = tables {
        return Ok(tables_vec)
    }

    Err(Error::new(
        ErrorKind::Other, "Could not get db rows."
    ).into())
}

fn setup_db () -> Result<()> {
    let connection = Connection::open(DB_PATH)?;
    connection.execute("
        CREATE TABLE if not exists MEASUREMENTS (
                timestamp int,
                topic varchar(255),
                value float
        )
        ",
        (),
    )?;

    connection.execute("
        CREATE TABLE if not exists LOGS (
                timestamp int,
                topic varchar(255),
                value varchar(255)
        )
        ",
        (),
    )?;
    Ok(())
}
