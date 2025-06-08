use std::{io::Error, path::Path, sync::{Arc, Mutex}};

use anyhow::Result;

use cursive::{
    theme::{BaseColor, Color, ColorStyle, Effect, Style}, view::Nameable, views::{Button, Dialog, DummyView, EditView, LinearLayout, ListView, TextView}, Cursive
};

use crate::{cli_args::ARGS, utils::SystemDService};

#[derive(Clone)]
enum FieldToUpdate {
    DBPath,
    BrokerIP,
    InstallLocation,
}

impl FieldToUpdate {
    fn into_title(&self) -> &str {
        match self {
            FieldToUpdate::DBPath =>            "DB Path:          ",
            FieldToUpdate::BrokerIP =>          "Broker IP:        ",
            FieldToUpdate::InstallLocation =>   "Install Location: ", 
        }
    }

    fn into_element_name(&self) -> &str {
        match self {
            FieldToUpdate::DBPath => "db_path_field",
            FieldToUpdate::BrokerIP => "broker_ip_field",
            FieldToUpdate::InstallLocation =>   "install_location_field", 
        }
    }

    fn get_default(&self) -> String {
        match self {
            FieldToUpdate::DBPath => {
                let full_path = Path::new(&ARGS.db_path).canonicalize();

                if let Ok(full_path) = full_path {
                    full_path.to_string_lossy().to_string()
                } else {
                    (&ARGS.db_path).to_owned()
                }
            } 
            FieldToUpdate::BrokerIP => (&ARGS.broker_ip).to_owned(),
            FieldToUpdate::InstallLocation => "/usr/local/home_automation".to_owned(),
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

trait ServiceDisplayRow {
    fn create_row(self) -> LinearLayout;
    fn get_element_name(&self) -> String;
}

impl ServiceDisplayRow for SystemDService {
    fn get_element_name(&self) -> String {
        format!("{}-status-text", self.service_name)
    }
    fn create_row(self) -> LinearLayout {
        // TODO: Listen for service state update.
        let element_name = Arc::new(self.get_element_name());
        let element_name_arc = element_name.clone();
        let service_state = Arc::new(Mutex::new(self));
        let service_state_arc = service_state.clone();

        let service_name = match service_state_arc.lock() {
            Ok(state) => {
                (*state.service_name).to_string()
            },
            Err(_e) => {
                "MUTEX_LOCK_FAIL".to_owned()
            },
        };

        let service_state_arc2 = service_state.clone();
        let initial_service_status = smol::block_on(async {
            if let Ok(state) = service_state_arc2.lock() {
                match (*state).check_unit_status().await {
                    Ok(res) => {
                        res
                    },
                    Err(e) => {
                        format!("{:?}", e)
                    },
                }
            } else {
                "MUTEX_LOCK_FAIL".to_owned()
            }
        });

        let service_name_ref = Arc::new(service_name);
        let service_name_ref1 = service_name_ref.clone();
        let service_name_ref2 = service_name_ref.clone();

        let service_state_arc = service_state.clone();
        let service_state_arc2 = service_state.clone();
        let service_state_arc3 = service_state.clone();

        let element_name_arc2 = element_name_arc.clone();
        let element_name_arc3 = element_name_arc.clone();
        LinearLayout::horizontal()
            .child(
                Dialog::around(
                    // Button Container
                    LinearLayout::horizontal()
                        // Buton Row
                        .child(
                            LinearLayout::vertical()
                                // Buttons:
                                .child(Button::new("Install", move |s| {
                                    let service_state_arc = service_state_arc.clone();
                                    let element_name_arc = element_name_arc.clone();

                                    // Collect all state from config boxes.
                                    // ----------------------------------------
                                    // DB PATH:
                                    let db_path = s.call_on_name(FieldToUpdate::DBPath.into_element_name(), |v: &mut TextView| {
                                        let content = v.get_content();
                                        content.source().to_owned()
                                    }).unwrap_or(FieldToUpdate::DBPath.get_default());

                                    // BROKER IP:
                                    let broker_ip = s.call_on_name(FieldToUpdate::BrokerIP.into_element_name(), |v: &mut TextView| {
                                        let content = v.get_content();
                                        content.source().to_owned()
                                    }).unwrap_or(FieldToUpdate::BrokerIP.get_default());

                                    // INSTALL PATH:
                                    let install_location = s.call_on_name(FieldToUpdate::InstallLocation.into_element_name(), |v: &mut TextView| {
                                        let content = v.get_content();
                                        content.source().to_owned()
                                    }).unwrap_or(FieldToUpdate::InstallLocation.get_default());
                                    // ----------------------------------------

                                    let res: Result<()> = smol::block_on(async {
                                        match service_state_arc.lock() {
                                            Ok(mut state) => {
                                                (*state).set_args(
                                                    match service_name_ref1.to_string().as_ref() {
                                                        "substore" => {
                                                            vec![
                                                                "--db-path".to_owned(), db_path,
                                                                "--broker-ip".to_owned(), broker_ip,
                                                            ]
                                                        },
                                                        _ => {
                                                            vec![]
                                                        }
                                                    }
                                                );
                                                (*state).set_install_location(&install_location);
                                                (*state).install_unit().await?;
                                                let new_unit_status = (*state).check_unit_status().await?;

                                                s.call_on_name(&element_name_arc.to_string(), | v: &mut TextView | {
                                                    v.set_content(new_unit_status.to_string())
                                                });
                                            },
                                            Err(_) => {
                                                return Err(
                                                    Error::new(std::io::ErrorKind::Other, "Poisoned mutex in install")
                                                        .into()
                                                );
                                            },
                                        };
                                        Ok(())
                                    });

                                    if let Err(e) = res {
                                        s.add_layer(Dialog::info(&format!("{:?}", e)));
                                    }
                                })
                                )
                                .child(Button::new("Uninstall", move |s| {
                                    let element_name_arc3 = element_name_arc3.clone();
                                    let res: Result<()> = smol::block_on(async {
                                        if let Ok(state) = service_state_arc2.lock() {
                                            (*state).uninstall_unit().await?;
                                            let new_unit_status = (*state).check_unit_status().await
                                                .unwrap_or_else(|e| {
                                                    format!("{:?}", e)
                                            });
                                            s.call_on_name(&element_name_arc3.to_string(), | v: &mut TextView | {
                                                v.set_content(new_unit_status.to_string());
                                            });
                                        };
                                        Ok(())
                                    });

                                    if let Err(e) = res {
                                        s.add_layer(Dialog::info(&format!("{:?}", e)));
                                    }
                                }))
                                .child(Button::new("Remove", move |s| {
                                    let res: Result<()> = smol::block_on(async {
                                        if let Ok(state) = service_state_arc3.lock() {
                                            (*state).remove_installed_files().await?;
                                        };
                                        Ok(())
                                    });

                                    if let Err(e) = res {
                                        s.add_layer(Dialog::info(&format!("{:?}", e)));
                                    }
                                }))
                        )
                        .child(DummyView)
                        // Buton Row
                        .child(
                            LinearLayout::vertical()
                                // Buttons:
                                .child(Button::new("Enable", |s| {}))
                                .child(Button::new("Disable", |s| {}))
                        )
                )
            )
            .child(Dialog::around(
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("SERVICE: "))
                            .child(TextView::new(service_name_ref2.to_string()))
                    )
                        
                    .child(
                    LinearLayout::horizontal()
                        .child(TextView::new("STATUS: "))
                        .child(TextView::new(&initial_service_status)
                            .with_name(element_name_arc2.to_string()))
                    )
            ))
    }
}

pub fn draw_config(s: &mut Cursive, main_menu_id: usize) {
    let config_row = ConfigRow::new(FieldToUpdate::DBPath).create_row();
    let config_row2 = ConfigRow::new(FieldToUpdate::BrokerIP).create_row();
    let config_row3 = ConfigRow::new(FieldToUpdate::InstallLocation).create_row();

    let substore_service_row = SystemDService::new(
        "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
        "substore".to_owned(),
        "sub_store".to_owned(),
        vec![
            "--db-path".to_owned(), FieldToUpdate::DBPath.get_default(),
            "--broker-ip".to_owned(), FieldToUpdate::BrokerIP.get_default(),
        ],
        Some("/usr/local/home_automation".to_owned()),
    ).create_row();

    s.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(
                    Dialog::around(
                        LinearLayout::vertical()
                            .child(Dialog::around(
                                LinearLayout::vertical()
                                    .child(config_row)
                                    .child(config_row2)
                                    .child(config_row3),
                            ))
                    )
                    .title("Configure"),
                )
                .child(
                    Dialog::around(
                        LinearLayout::vertical()
                            .child(Dialog::around(
                                ListView::new().child("-->", substore_service_row)
                                // service_row
                            )
                                .title("Install Services"))
                            .child(
                                Dialog::around(TextView::new("TODO")).title("Check Dependencies"),
                            ),
                    )
                    .title("Services"),
                )
                .child(Button::new("MAIN MENU", move |s| {
                    s.set_screen(main_menu_id);
                })),
        )
        .title("CONFIGURATION"),
    );
}
