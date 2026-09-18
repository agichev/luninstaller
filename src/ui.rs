use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, CssProvider, Dialog, HeaderBar, IconLookupFlags,
    IconTheme, Image, Label, ListBox, ListBoxRow, Orientation, ResponseType,
    ScrolledWindow, SearchEntry, Spinner, StyleContext, Window, WindowType,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

use crate::app_scanner::{scan_applications, AppEntry};
use crate::uninstaller::{uninstall_app, UninstallResult};

pub fn build_ui(application: &gtk::Application) {
    load_css();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Application Uninstaller");
    window.set_default_size(780, 680);
    window.set_position(gtk::WindowPosition::Center);
    application.add_window(&window);

    // HeaderBar
    let header_bar = HeaderBar::new();
    header_bar.set_show_close_button(true);
    header_bar.set_title(Some("Application Uninstaller"));
    header_bar.set_subtitle(Some("GNOME Menu Applications"));

    let refresh_button = Button::from_icon_name(Some("view-refresh-symbolic"), gtk::IconSize::Button);
    refresh_button.set_tooltip_text(Some("Refresh list"));
    header_bar.pack_start(&refresh_button);

    window.set_titlebar(Some(&header_bar));

    // Root container
    let root_box = GtkBox::new(Orientation::Vertical, 12);
    root_box.set_margin_top(14);
    root_box.set_margin_bottom(14);
    root_box.set_margin_start(18);
    root_box.set_margin_end(18);

    // Search bar
    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search installed applications..."));
    root_box.pack_start(&search_entry, false, false, 0);

    // List view in ScrolledWindow
    let scrolled_window = ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    scrolled_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled_window.set_vexpand(true);

    let list_box = ListBox::new();
    list_box.set_selection_mode(gtk::SelectionMode::None);
    list_box.style_context().add_class("app-list-box");
    scrolled_window.add(&list_box);

    root_box.pack_start(&scrolled_window, true, true, 0);

    // Status / Count bar
    let status_label = Label::new(None);
    status_label.set_xalign(0.0);
    status_label.style_context().add_class("dim-label");
    root_box.pack_start(&status_label, false, false, 0);

    window.add(&root_box);

    // State
    let apps_state: Rc<RefCell<Vec<AppEntry>>> = Rc::new(RefCell::new(Vec::new()));

    // Closure to populate list
    let populate_list = {
        let list_box = list_box.clone();
        let status_label = status_label.clone();
        let apps_state = apps_state.clone();
        let window_clone = window.clone();

        move || {
            for child in list_box.children() {
                list_box.remove(&child);
            }

            let apps = scan_applications();
            *apps_state.borrow_mut() = apps.clone();

            status_label.set_text(&format!("Applications found: {}", apps.len()));

            let icon_theme = IconTheme::default().unwrap_or_else(IconTheme::new);

            for app in &apps {
                let row = create_app_row(
                    app,
                    &icon_theme,
                    &window_clone,
                    apps_state.clone(),
                    list_box.clone(),
                    status_label.clone(),
                );
                list_box.add(&row);
            }

            list_box.show_all();
        }
    };

    // Filter closure
    let filter_func = {
        let apps_state = apps_state.clone();
        let search_entry_clone = search_entry.clone();

        move |row: &ListBoxRow| -> bool {
            let query = search_entry_clone.text().to_lowercase();
            if query.trim().is_empty() {
                return true;
            }

            let index = row.index() as usize;
            let apps = apps_state.borrow();
            if let Some(app) = apps.get(index) {
                app.name.to_lowercase().contains(&query)
                    || app.comment.to_lowercase().contains(&query)
                    || app.package_type.badge_label().to_lowercase().contains(&query)
                    || app.package_type.details().to_lowercase().contains(&query)
            } else {
                true
            }
        }
    };

    list_box.set_filter_func(Some(Box::new(filter_func)));

    // Connect search entry
    {
        let list_box = list_box.clone();
        search_entry.connect_search_changed(move |_| {
            list_box.invalidate_filter();
        });
    }

    // Connect refresh button
    {
        let populate_list = populate_list.clone();
        refresh_button.connect_clicked(move |_| {
            populate_list();
        });
    }

    // Initial population
    populate_list();

    window.show_all();
}

fn create_app_row(
    app: &AppEntry,
    icon_theme: &IconTheme,
    parent_window: &Window,
    apps_state: Rc<RefCell<Vec<AppEntry>>>,
    list_box: ListBox,
    status_label: Label,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.style_context().add_class("app-row");

    let h_box = GtkBox::new(Orientation::Horizontal, 14);
    h_box.set_margin_top(8);
    h_box.set_margin_bottom(8);
    h_box.set_margin_start(12);
    h_box.set_margin_end(12);

    // App Icon
    let image = create_app_icon(&app.icon, icon_theme);
    h_box.pack_start(&image, false, false, 0);

    // App Info VBox
    let info_box = GtkBox::new(Orientation::Vertical, 4);
    info_box.set_valign(Align::Center);

    let title_badge_box = GtkBox::new(Orientation::Horizontal, 8);

    let name_label = Label::new(Some(&app.name));
    name_label.set_xalign(0.0);
    name_label.style_context().add_class("app-title");
    title_badge_box.pack_start(&name_label, false, false, 0);

    let badge_label = Label::new(Some(app.package_type.badge_label()));
    badge_label.style_context().add_class("badge");
    badge_label.style_context().add_class(app.package_type.badge_css_class());
    title_badge_box.pack_start(&badge_label, false, false, 0);

    info_box.pack_start(&title_badge_box, false, false, 0);

    let subtitle = if !app.comment.is_empty() {
        app.comment.clone()
    } else {
        app.package_type.details()
    };
    let desc_label = Label::new(Some(&subtitle));
    desc_label.set_xalign(0.0);
    desc_label.set_ellipsize(pango::EllipsizeMode::End);
    desc_label.style_context().add_class("app-subtitle");
    info_box.pack_start(&desc_label, false, false, 0);

    h_box.pack_start(&info_box, true, true, 0);

    // Uninstall button
    let delete_btn = Button::new();
    let btn_content = GtkBox::new(Orientation::Horizontal, 6);
    let trash_icon = Image::from_icon_name(Some("user-trash-symbolic"), gtk::IconSize::Button);
    let btn_label = Label::new(Some("Uninstall"));
    btn_content.pack_start(&trash_icon, false, false, 0);
    btn_content.pack_start(&btn_label, false, false, 0);
    delete_btn.add(&btn_content);

    delete_btn.set_valign(Align::Center);
    delete_btn.style_context().add_class("destructive-action");
    delete_btn.set_tooltip_text(Some("Uninstall application"));

    // Connect delete button
    {
        let app_clone = app.clone();
        let parent_clone = parent_window.clone();
        let apps_state = apps_state.clone();
        let list_box = list_box.clone();
        let status_label = status_label.clone();

        delete_btn.connect_clicked(move |_| {
            confirm_and_uninstall(
                &app_clone,
                &parent_clone,
                apps_state.clone(),
                list_box.clone(),
                status_label.clone(),
            );
        });
    }

    h_box.pack_start(&delete_btn, false, false, 0);

    row.add(&h_box);
    row
}

fn create_app_icon(icon_name: &str, icon_theme: &IconTheme) -> Image {
    let size = 48;

    if !icon_name.is_empty() {
        if icon_name.starts_with('/') {
            if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file_at_scale(icon_name, size, size, true) {
                return Image::from_pixbuf(Some(&pixbuf));
            }
        }

        if icon_theme.has_icon(icon_name) {
            if let Some(icon_info) = icon_theme.lookup_icon(icon_name, size, IconLookupFlags::FORCE_SIZE) {
                if let Ok(pixbuf) = icon_info.load_icon() {
                    return Image::from_pixbuf(Some(&pixbuf));
                }
            }
        }
    }

    Image::from_icon_name(Some("application-x-executable"), gtk::IconSize::Dialog)
}

fn confirm_and_uninstall(
    app: &AppEntry,
    parent: &Window,
    apps_state: Rc<RefCell<Vec<AppEntry>>>,
    list_box: ListBox,
    status_label: Label,
) {
    let dialog = Dialog::with_buttons(
        Some("Confirm Uninstall"),
        Some(parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Uninstall", ResponseType::Accept),
        ],
    );

    dialog.set_default_size(440, 200);

    let content_area = dialog.content_area();
    content_area.set_margin_top(20);
    content_area.set_margin_bottom(20);
    content_area.set_margin_start(24);
    content_area.set_margin_end(24);
    content_area.set_spacing(12);

    let title_label = Label::new(None);
    title_label.set_markup(&format!(
        "<b>Are you sure you want to uninstall \"{}\"?</b>",
        glib::markup_escape_text(&app.name)
    ));
    title_label.set_xalign(0.0);
    content_area.pack_start(&title_label, false, false, 0);

    let info_text = format!(
        "Type: {}\n{}\nPath: {}",
        app.package_type.badge_label(),
        app.package_type.details(),
        app.desktop_path.display()
    );
    let info_label = Label::new(Some(&info_text));
    info_label.set_xalign(0.0);
    info_label.style_context().add_class("dim-label");
    content_area.pack_start(&info_label, false, false, 0);

    if let Some(accept_btn) = dialog.widget_for_response(ResponseType::Accept) {
        accept_btn.style_context().add_class("destructive-action");
    }

    dialog.show_all();

    let app_clone = app.clone();
    let parent_clone = parent.clone();
    let dialog_clone = dialog.clone();

    dialog.connect_response(move |d, response| {
        d.hide();
        dialog_clone.close();

        if response == ResponseType::Accept {
            perform_uninstall_async(
                &app_clone,
                &parent_clone,
                apps_state.clone(),
                list_box.clone(),
                status_label.clone(),
            );
        }
    });
}

fn perform_uninstall_async(
    app: &AppEntry,
    parent: &Window,
    apps_state: Rc<RefCell<Vec<AppEntry>>>,
    list_box: ListBox,
    status_label: Label,
) {
    let progress_dialog = Dialog::with_buttons(
        Some("Uninstalling..."),
        Some(parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[],
    );
    progress_dialog.set_default_size(360, 140);
    let content = progress_dialog.content_area();
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_spacing(16);

    let hbox = GtkBox::new(Orientation::Horizontal, 16);
    let spinner = Spinner::new();
    spinner.start();
    let msg_label = Label::new(Some(&format!("Uninstalling '{}'...", app.name)));
    hbox.pack_start(&spinner, false, false, 0);
    hbox.pack_start(&msg_label, true, true, 0);
    content.pack_start(&hbox, true, true, 0);

    progress_dialog.show_all();

    let (sender, receiver) = mpsc::channel::<UninstallResult>();
    let app_clone = app.clone();

    thread::spawn(move || {
        let result = uninstall_app(&app_clone);
        let _ = sender.send(result);
    });

    let parent_clone = parent.clone();
    let progress_dialog_clone = progress_dialog.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(result) => {
                progress_dialog_clone.hide();
                progress_dialog_clone.close();

                if result.success {
                    for child in list_box.children() {
                        list_box.remove(&child);
                    }
                    let apps = scan_applications();
                    *apps_state.borrow_mut() = apps.clone();
                    status_label.set_text(&format!("Applications found: {}", apps.len()));

                    let icon_theme = IconTheme::default().unwrap_or_else(IconTheme::new);
                    for app_item in &apps {
                        let row = create_app_row(
                            app_item,
                            &icon_theme,
                            &parent_clone,
                            apps_state.clone(),
                            list_box.clone(),
                            status_label.clone(),
                        );
                        list_box.add(&row);
                    }
                    list_box.show_all();
                }

                let res_dialog = Dialog::with_buttons(
                    Some(if result.success { "Success" } else { "Error" }),
                    Some(&parent_clone),
                    gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
                    &[("OK", ResponseType::Ok)],
                );
                res_dialog.set_default_size(360, 140);
                let content = res_dialog.content_area();
                content.set_margin_top(20);
                content.set_margin_bottom(20);
                content.set_margin_start(24);
                content.set_margin_end(24);

                let label = Label::new(Some(&result.message));
                label.set_line_wrap(true);
                content.pack_start(&label, true, true, 0);

                res_dialog.show_all();
                res_dialog.connect_response(|d, _| {
                    d.hide();
                    d.close();
                });

                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                progress_dialog_clone.hide();
                progress_dialog_clone.close();
                glib::ControlFlow::Break
            }
        }
    });
}

fn load_css() {
    let provider = CssProvider::new();
    let css = "
    .app-list-box {
        background-color: transparent;
    }

    .app-row {
        background-color: alpha(@theme_base_color, 0.08);
        border: 1px solid alpha(@theme_fg_color, 0.12);
        border-radius: 10px;
        margin-bottom: 6px;
        transition: all 120ms ease;
    }

    .app-row:hover {
        background-color: alpha(@theme_base_color, 0.16);
        border-color: alpha(@theme_fg_color, 0.22);
    }

    .app-title {
        font-size: 14.5px;
        font-weight: 600;
        color: @theme_text_color;
    }

    .app-subtitle {
        font-size: 12px;
        opacity: 0.75;
    }

    .dim-label {
        font-size: 12px;
        opacity: 0.65;
    }

    .badge {
        font-size: 10.5px;
        font-weight: 600;
        padding: 2px 8px;
        border-radius: 6px;
    }

    .badge-flatpak {
        background-color: alpha(#3584e4, 0.22);
        color: #62a0ea;
        border: 1px solid alpha(#3584e4, 0.4);
    }

    .badge-snap {
        background-color: alpha(#e66100, 0.22);
        color: #ff7800;
        border: 1px solid alpha(#e66100, 0.4);
    }

    .badge-deb {
        background-color: alpha(#9141ac, 0.22);
        color: #c061cb;
        border: 1px solid alpha(#9141ac, 0.4);
    }

    .badge-local {
        background-color: alpha(@theme_fg_color, 0.12);
        color: @theme_text_color;
        border: 1px solid alpha(@theme_fg_color, 0.25);
    }

    .destructive-action {
        background-color: alpha(#c01c28, 0.2);
        color: #f66151;
        border: 1px solid alpha(#c01c28, 0.4);
        border-radius: 8px;
        font-weight: 600;
        padding: 6px 14px;
        transition: all 120ms ease;
    }

    .destructive-action:hover {
        background-color: #c01c28;
        color: #ffffff;
        border-color: #c01c28;
    }
    ";

    if let Err(e) = provider.load_from_data(css.as_bytes()) {
        eprintln!("Failed to load CSS styles: {}", e);
    }

    if let Some(screen) = gdk::Screen::default() {
        StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
