use cursive::{
    view::{Nameable, Resizable},
    views::{
        Dialog,
        FixedLayout,
        Layer,
        NamedView,
        OnLayoutView,
        TextView
    },
    Cursive,
    Rect,
    Vec2,
    View,
    XY
};

pub fn quit (s: &mut Cursive) {
    s.quit()
}

pub fn info (s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("general_info", |_v: &mut Dialog| {}) {
        s.pop_layer();
    };
    s.add_layer(Dialog::text("-> <t> for tables").title("INSTRUCTIONS:").with_name("general_info"));
}

pub fn show_help (s: &mut Cursive) {
    if let Some(_) = s.call_on_name("help_menu", |_v: &mut NamedView<Dialog>| {}) {
        s.pop_layer();
    };

    s.add_layer(Dialog::info("TODO! Write info").title("HELP").with_name("help_menu"));
}


pub fn draw_bottom_bar (s: &mut Cursive) {
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

