use std::{fmt::Display, io::{Error, ErrorKind}, sync::{Arc, Mutex}};

use color_eyre::Result;
use rusqlite::{params, Connection};
use cursive::{
    view::{Nameable, Resizable, Scrollable}, views::{
        Button, Dialog, DummyView, FixedLayout, Layer, LinearLayout, NamedView, OnLayoutView, ScrollView, SelectView, TextView
    }, Cursive, CursiveExt, Rect, Vec2, View, XY
};

static DB_PATH: &str = "./data.db";

fn main() -> Result<()> {
    setup_db()?;
    let mut siv = Cursive::new();
    draw_bottom_bar(&mut siv);

    siv.add_layer(TextView::new(""));
    info(&mut siv);

    siv.add_global_callback('q', quit);
    siv.add_global_callback('p', pop_top);
    siv.add_global_callback('i', info);
    siv.add_global_callback('t', show_tables);

    siv.add_global_callback('h', show_help);

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
    if let Some(_) = s.call_on_name("general_info", |_v: &mut Dialog| {}) {
        s.pop_layer();
    };
    s.add_layer(Dialog::text("-> <t> for tables").title("INSTRUCTIONS:").with_name("general_info"));
}

fn show_tables (s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("tables_list", |_v: &mut Dialog| {}) {
        s.pop_layer();
    };

    if let Ok(tables) = get_tables() {
        let mut list = LinearLayout::vertical();
        tables.iter().for_each(|table_name| {
            let table_name_clone = table_name.clone();
            list.add_child(Button::new(table_name, move |s| {
                draw_readings_for_table(s, &table_name_clone);
            }));
        });


        s.add_layer(Dialog::around(list).with_name("tables_list"));
    } else {
        s.add_layer(
            Dialog::around(
                TextView::new("Tables not found.")
            ).with_name("tables_list")
        );
    }

}

fn pop_top (s: &mut Cursive) {
    s.pop_layer();
}

fn draw_readings_for_table (s: &mut Cursive, table_name: &str) -> Result<()> {
    s.pop_layer();

    let selected_row = Arc::new(Mutex::new(Option::<DBRow>::None));

    let selected_row_clone = selected_row.clone();
    let table_name_cp = table_name.to_owned();
    let buttons = LinearLayout::vertical()
        .child(Button::new("DELETE", move |s| { 
            if let Ok(selected_row) = selected_row_clone.lock() {
                let inspected =  selected_row.clone().inspect(|row| {
                    match delete_row_from_table(row, &table_name_cp) {
                        Ok(rows_changed) => {
                            s.add_layer(Dialog::info(format!("{} rows deleted: {}", rows_changed, row)));
                            if let Err(e) = update_table(s, &table_name_cp) {
                                s.add_layer(Dialog::info(format!("Something went wrong {}", e)));
                            };
                        },
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Something went wrong {}", e)));
                        },
                    };
                });
                if let None = inspected {
                    s.add_layer(Dialog::info("No rows selected."));
                }
            } else {
                s.add_layer(Dialog::info("Failed to lock mutex."));
            }
        }).with_name("db_helper_button")
        ).child(Button::new("CANCEL", |_s| { }));

    let selected_row_clone = selected_row.clone();
    let select_view = SelectView::<DBRow>::new()
        .on_select( move |s, row| {
            if let Ok(mut selected_row) = selected_row_clone.lock() {
                *selected_row = Some(row.to_owned());
            } else {
                s.add_layer(Dialog::info("Failed to lock mutex."));
            }
        })
        .on_submit(move |s, _row| {
            if let Err(_) = s.focus_name("db_helper_button") {
                s.add_layer(Dialog::info("View not found."));
            }
        });

    let select_view = select_view.with_name("main_table").scrollable();
    s.add_layer(Dialog::around(
        LinearLayout::horizontal()
            .child(buttons)
            .child(DummyView)
            .child(select_view)
    ));

    update_table(s, table_name)?;
    Ok(())
}

fn update_table(s: &mut Cursive, table_name: &str) -> Result<()> {
    let res = s.call_on_name("main_table", |v: &mut SelectView<DBRow>| -> Result<()> {
        let rows = get_all_from_table(table_name)?;
        v.clear();
        v.add_all(rows.iter().map(|row| {
            (row, row.to_owned())
        }));
        Ok(())
    });

    if let Some(res) = res {
        res?;
    } else {
        s.add_layer(Dialog::info("Something went wrong."));
    }
    Ok(())
}

fn show_help (s: &mut Cursive) {
    if let Some(_) = s.call_on_name("help_menu", |_v: &mut NamedView<Dialog>| {}) {
        s.pop_layer();
    };

    s.add_layer(Dialog::info("TODO! Write info").title("HELP").with_name("help_menu"));
}

fn draw_bottom_bar (s: &mut Cursive) {
    s.screen_mut()
        .add_transparent_layer(
            OnLayoutView::new(
                FixedLayout::new()
                    .child(
                        Rect::from_point(Vec2::zero()),
                        Layer::new(
                            TextView::new("<T>ables | <I>nfo | <H>elp | <P>op top | <Q>uit ")
                    ).full_width()
                    ),
                draw_bottom_bar_cb
            ).full_screen().with_name("bottom_bar")
        );

    s.add_layer(TextView::new(""));
}

fn draw_bottom_bar_cb (layout: &mut FixedLayout, size: XY<usize>) {
    let rect = cursive::Rect::from_size((0, size.y - 1), (size.x, 1));
    layout.set_child_position(0, rect);
    layout.layout(size);
}

struct DBRow {
    timestamp: u64,
    topic: String,
    value: String,
}

impl From<&DBRow> for String {
    fn from(value: &DBRow) -> Self {
        format!("{} -> {} -> {}", value.timestamp, value.value, value.topic)
    }
}

impl Clone for DBRow {
    fn clone(&self) -> Self {
        DBRow {
           timestamp: self.timestamp,
           topic: self.topic.to_owned(),
           value: self.value.to_owned()
        }
    }
}

impl Display for DBRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} -> {} -> {}", self.timestamp, self.value, self.topic))
    }
}

fn delete_row_from_table (row: &DBRow, table_name: &str) -> Result<usize> {
    let conn = Connection::open(DB_PATH)?;
    let query = format!(
        "DELETE FROM {} WHERE timestamp = ?1 and topic = ?2 and value = ?3;",
        table_name
    );

    let rows_changed = conn.execute(&query, params![row.timestamp, row.topic, row.value])?;

    Ok(rows_changed)
}

fn get_all_from_table (table_name: &str) -> Result<Vec<DBRow>> {
    let conn = Connection::open(DB_PATH)?;
    let mut statement = conn.prepare(&format!("SELECT * FROM {};", table_name))?;

    let rows_iter = statement.query_map([], |row| {
        let val = row.get::<usize, f32>(2).unwrap_or(0.);
        let curr_row = DBRow{
            timestamp: row.get(0).unwrap_or(0),
            topic: row.get(1).unwrap_or("".to_owned()),
            value: format!("{:.2}", val),
        };
        Ok(curr_row)
    })?;

    let rows: Result<Vec<DBRow>, rusqlite::Error> = rows_iter.into_iter().collect();

    if let Ok(valid_rows) = rows {
        return Ok(valid_rows)
    }

    return Err(Error::new(ErrorKind::Other, "Something went wrong while getting data from the table").into());
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
