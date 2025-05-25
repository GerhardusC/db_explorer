use std::sync::{Arc, Mutex};

use color_eyre::Result;
use cursive::{
    view::{Nameable, Scrollable}, views::{
        Button, Dialog, DummyView, EditView, LinearLayout, NamedView, ScrollView, SelectView, TextView
    }, Cursive
};
use crate::db_interactions::{get_tables, get_all_from_table, DBRow, delete_row_from_table};

pub fn draw_db_explorer (s: &mut Cursive) {
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

fn draw_table (s: &mut Cursive, table_name: &str) -> Result<()> {
    s.pop_layer();

    let selected_row = Arc::new(Mutex::new(Option::<DBRow>::None));
    let val_filter = Arc::new(Mutex::new(String::new()));

    let buttons = create_buttons(selected_row.clone(), val_filter.clone(), table_name);
    let row_container = create_row_container(selected_row);

    s.add_layer(Dialog::around(
        LinearLayout::horizontal()
            .child(buttons)
            .child(DummyView)
            .child(row_container)
    ));

    let val_filter = val_filter.clone();
    update_table(s, table_name, val_filter)
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

fn create_buttons (selected_row: Arc<Mutex<Option<DBRow>>>, val_filter: Arc<Mutex<String>>, table_name: &str) -> LinearLayout {
    let table_name_cp = table_name.to_owned();
    let table_name_cp_cp = table_name.to_owned();

    // Passed to handle filter
    let val_filter_for_filter = val_filter.clone();

    LinearLayout::vertical()
        .child(Button::new("FILTER", move |s| {
            handle_filter_db_rows(s, val_filter_for_filter.clone(), &table_name_cp);
        }))
        .child(Button::new("DELETE", move |s| { 
            // Passed a reference of val_filter to handle delete, becuase table needs to be updated.
            handle_delete_db_row(s, selected_row.clone(), val_filter.clone(), &table_name_cp_cp);
        }).with_name("db_helper_button"))
        // .child(Button::new("CANCEL", |_s| {}))
}

fn handle_filter_db_rows (s: &mut Cursive, val_filter: Arc<Mutex<String>>, table_name: &str) {
    let table_name_cp_for_update = table_name.to_owned();
    let val_filter_cp = val_filter.clone(); // For on Edit to update value
    let val_filter_cp_cp = val_filter.clone(); // For on Submit to update value
    s.add_layer(Dialog::around(
        EditView::new()
            .on_edit(move |s, val, _| {
                if let Ok(mut val_filter) = val_filter_cp.lock() {
                    *val_filter = val.to_owned();
                } else {
                    s.add_layer(Dialog::info("Something went wrong on edit."));
                };
            })
            .on_submit(move |s, val| {
                if let Ok(mut val_filter) = val_filter_cp_cp.lock() {
                    *val_filter = val.to_owned();
                } else {
                    s.add_layer(Dialog::info("Something went wrong on submission."));
                };
            }))
        .title("Enter Value Filter: ")
        .button("OK", move |s| {
            let val_filter = val_filter.clone(); // For updating table
            if let Err(e) = update_table(s, &table_name_cp_for_update, val_filter) {
                s.add_layer(Dialog::info(format!("Something went wrong on submission: {}", e)));
            } else {
                s.pop_layer();
            };
        })
        .button("CANCEL", |s| {
            s.pop_layer();
        }))

}

fn handle_delete_db_row(s: &mut Cursive, selected_row: Arc<Mutex<Option<DBRow>>>, val_filter: Arc<Mutex<String>>, table_name: &str) {
    match selected_row.lock() {
        Ok(mut selected_row) => {
            let inspected =  selected_row.clone().inspect(|row| {
                match delete_row_from_table(row, &table_name) {
                    Ok(rows_changed) => {
                        s.add_layer(Dialog::info(format!("{} rows deleted: {}", rows_changed, row)));
                        // Filter is referenced here to update the table. This needs to read the filter to know what to render.
                        if let Err(e) = update_table(s, table_name, val_filter) {
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

fn update_table(s: &mut Cursive, table_name: &str, val_filter: Arc<Mutex<String>>) -> Result<()> {
    let res = s.call_on_name("main_table", |v: &mut SelectView<DBRow>| -> Result<()> {
        if let Ok(val_filter) = val_filter.lock() {
            let rows = get_all_from_table(table_name)?;
            v.clear();
            v.add_all(rows.iter()
                .filter(|row| row.value.contains(&val_filter.to_string()))
                .map(|row| (row, row.to_owned())
            ));
        };
        Ok(())
    });

    if let Some(res) = res {
        res?;
    } else {
        s.add_layer(Dialog::info("Something went wrong."));
    }
    Ok(())
}
