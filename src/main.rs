mod app_scanner;
mod ui;
mod uninstaller;

use gio::prelude::*;
use gtk::Application;

fn main() {
    let app = Application::new(
        Some("com.github.agichev.luninstaller"),
        gio::ApplicationFlags::FLAGS_NONE,
    );

    app.connect_activate(|app| {
        ui::build_ui(app);
    });

    app.run();
}
