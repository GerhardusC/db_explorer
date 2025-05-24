use std::sync::{Arc, Mutex};

use color_eyre::Result;
use cursive::{
    view::{Nameable, Scrollable}, views::{
        Button, Dialog, DummyView, LinearLayout, NamedView, ScrollView, SelectView, TextView
    }, Cursive
};
use crate::db_interactions::{get_tables, get_all_from_table, DBRow, delete_row_from_table};

pub fn init_table_selection (s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("tables_list", |_v: &mut Dialog| {}) {
        s.pop_layer();
    };

    if let Ok(tables) = get_tables() {
        let mut list = LinearLayout::vertical();
        tables.iter().for_each(|table_name| {
            let table_name_clone = table_name.clone();
            list.add_child(Button::new(table_name, move |s| {
                if let Err(e) = draw_table(s, &table_name_clone) {
                    s.add_layer(Dialog::info(format!("Something went wrong {}", e)));
                };
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

fn create_buttons (selected_row: Arc<Mutex<Option<DBRow>>>, table_name: &str) -> LinearLayout {
    let table_name_cp = table_name.to_owned();
    LinearLayout::vertical()
        .child(Button::new("DELETE", move |s| { 
            let selected_row_clone = selected_row.clone();
            handle_delete_db_row(s, selected_row_clone, &table_name_cp);
        }).with_name("db_helper_button")
        ).child(Button::new("CANCEL", |_s| { }))
}

fn create_row_container (selected_row: Arc<Mutex<Option<DBRow>>>) -> ScrollView<NamedView<SelectView<DBRow>>> {
    let selected_row_clone = selected_row.clone();
    let selected_row_submit_clone = selected_row.clone();
    SelectView::<DBRow>::new()
        .on_select( move |s, row| {
            if let Ok(mut selected_row) = selected_row_clone.lock() {
                *selected_row = Some(row.to_owned());
            } else {
                s.add_layer(Dialog::info("Failed to lock mutex."));
            }
        })
        .on_submit(move |s, row| {
            if let Err(_) = s.focus_name("db_helper_button") {
                s.add_layer(Dialog::info("View not found."));
            }
            if let Ok(mut selected_row) = selected_row_submit_clone.lock() {
                *selected_row = Some(row.to_owned());
            }
        }).with_name("main_table").scrollable()
}

fn handle_delete_db_row(s: &mut Cursive, selected_row: Arc<Mutex<Option<DBRow>>>, table_name: &str) {
    match selected_row.lock() {
        Ok(mut selected_row) => {
            let inspected =  selected_row.clone().inspect(|row| {
                match delete_row_from_table(row, &table_name) {
                    Ok(rows_changed) => {
                        s.add_layer(Dialog::info(format!("{} rows deleted: {}", rows_changed, row)));
                        if let Err(e) = update_table(s, table_name) {
                            s.add_layer(Dialog::info(format!("Something went wrong {}", e)));
                        };
                        *selected_row = None;
                    },
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Something went wrong {}", e)));
                    },
                };
            });
            if let None = inspected {
                s.add_layer(Dialog::info("No rows selected."));
            }
        },
        Err(_) => {
            s.add_layer(Dialog::info("Failed to lock mutex."));
        },
    }
}

fn draw_table (s: &mut Cursive, table_name: &str) -> Result<()> {
    s.pop_layer();

    let selected_row = Arc::new(Mutex::new(Option::<DBRow>::None));

    let buttons = create_buttons(selected_row.clone(), table_name);
    let row_container = create_row_container(selected_row);

    s.add_layer(Dialog::around(
        LinearLayout::horizontal()
            .child(buttons)
            .child(DummyView)
            .child(row_container)
    ));

    update_table(s, table_name)
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
