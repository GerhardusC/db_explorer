use cursive::{
    Cursive,
    views::{Button, Dialog, LinearLayout, TextView},
};

pub fn draw_config(s: &mut Cursive, main_menu_id: usize) {
    s.add_layer(
        Dialog::around(
            LinearLayout::horizontal()
                .child(
                    Dialog::around(
                        LinearLayout::vertical()
                            .child(Dialog::around(TextView::new("TODO")).title("Configure"))
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
