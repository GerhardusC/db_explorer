use cursive::{
    Cursive,
    views::{Dialog, TextView},
};

pub fn check_config(s: &mut Cursive) {
    s.add_layer(TextView::new(""));
    if let Err(e) = s.load_toml(include_str!("./config.toml")) {
        s.add_layer(Dialog::info(format!("{:?}", e)));
    };
}

pub fn quit(s: &mut Cursive) {
    s.quit()
}
