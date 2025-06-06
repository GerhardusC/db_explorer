use std::sync::{Arc, Mutex};

use cursive::{
    Cursive,
    theme::{BaseColor, Color, ColorStyle, Effect, Style},
    view::Nameable,
    views::{Button, Dialog, EditView, LinearLayout, TextView},
};

use crate::cli_args::ARGS;

#[derive(Clone)]
enum FieldToUpdate {
    DBPath,
    BrokerIP,
}

impl FieldToUpdate {
    fn into_title(&self) -> &str {
        match self {
            FieldToUpdate::DBPath => "DB Path:   ",
            FieldToUpdate::BrokerIP => "Broker IP: ",
        }
    }

    fn into_element_name(&self) -> &str {
        match self {
            FieldToUpdate::DBPath => "db_path_field",
            FieldToUpdate::BrokerIP => "broker_ip_field",
        }
    }

    fn get_default(&self) -> String {
        match self {
            FieldToUpdate::DBPath => (&ARGS.db_path).to_owned(),
            FieldToUpdate::BrokerIP => (&ARGS.broker_ip).to_owned(),
        }
    }
}

#[derive(Clone)]
struct ConfigRow {
    field_to_update: FieldToUpdate,
}

impl ConfigRow {
    fn new(field_to_update: FieldToUpdate) -> Self {
        ConfigRow { field_to_update }
    }

    fn create_row(&self) -> LinearLayout {
        let field_arc = Arc::new(self.field_to_update.clone());
        let field1 = field_arc.clone();
        let field2 = field_arc.clone();

        let field_title = self.field_to_update.into_title().to_owned();

        let row_value = Arc::new(Mutex::new(self.field_to_update.get_default()));
        let row_value1 = row_value.clone();
        LinearLayout::horizontal()
            .child(Button::new("EDIT", move |s| {
                let field1 = field1.clone();
                let field2 = field1.clone();
                let row_value1 = row_value1.clone();
                let row_value2 = row_value1.clone();
                let row_value3 = row_value1.clone();
                let row_value4 = row_value1.clone();
                let row_value5 = row_value1.clone();
                s.add_layer(
                    Dialog::around(
                        EditView::new()
                            .on_submit(move |s, val| {
                                if let Ok(mut row_value) = row_value4.lock() {
                                    *row_value = val.to_owned();
                                }
                                let row_value5 = row_value5.clone();
                                s.call_on_name(
                                    field2.into_element_name(),
                                    move |v: &mut TextView| {
                                        if let Ok(new_row_value) = row_value5.lock() {
                                            let val = (*new_row_value).to_owned();
                                            v.set_content(&val);
                                        }
                                    },
                                );
                                s.pop_layer();
                            })
                            .on_edit(move |_s, val, _i| {
                                if let Ok(mut row_value) = row_value1.lock() {
                                    *row_value = val.to_owned();
                                }
                            }),
                    )
                    .title(&field_title)
                    .button("OK", move |s| {
                        let row_value2 = row_value2.clone();
                        s.call_on_name(field1.into_element_name(), move |v: &mut TextView| {
                            if let Ok(new_row_value) = row_value2.lock() {
                                let val = (*new_row_value).to_owned();
                                v.set_content(&val);
                            }
                        });
                        s.pop_layer();
                    })
                    .button("CANCEL", move |s| {
                        if let Ok(mut row_value) = row_value3.lock() {
                            *row_value = "".to_owned();
                        }
                        s.pop_layer();
                    }),
                );
            }))
            .child(
                TextView::new(field2.into_title().to_owned())
                    .style(Style::from(Effect::Bold))
                    .style(Style::from(ColorStyle::new(
                        Color::Dark(BaseColor::White),
                        Color::Dark(BaseColor::Green),
                    ))),
            )
            .child(
                TextView::new(field_arc.clone().get_default())
                    .with_name(field_arc.clone().into_element_name()),
            )
    }
}

pub fn draw_config(s: &mut Cursive, main_menu_id: usize) {
    let config_row = ConfigRow::new(FieldToUpdate::DBPath).create_row();
    let config_row2 = ConfigRow::new(FieldToUpdate::BrokerIP).create_row();

    s.add_layer(
        Dialog::around(
            LinearLayout::horizontal()
                .child(
                    Dialog::around(
                        LinearLayout::vertical()
                            .child(Dialog::around(
                                LinearLayout::vertical()
                                    .child(config_row)
                                    .child(config_row2),
                            ))
                            .child(Button::new("MAIN MENU", move |s| {
                                s.set_screen(main_menu_id);
                            })),
                    )
                    .title("Configure"),
                )
                .child(
                    Dialog::around(
                        LinearLayout::vertical()
                            .child(Dialog::around(TextView::new("TODO")).title("Install Services"))
                            .child(
                                Dialog::around(TextView::new("TODO")).title("Check Dependencies"),
                            ),
                    )
                    .title("Services"),
                ),
        )
        .title("CONFIGURATION"),
    );
}
