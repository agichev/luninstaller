use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageType {
    Flatpak(String), // App ID (e.g. org.mozilla.firefox)
    Snap(String),    // Snap name (e.g. discord)
    Deb(String),     // Deb package name (e.g. brave-browser)
    Local,           // Custom/local desktop file in ~/.local or unmanaged
}

impl PackageType {
    pub fn badge_label(&self) -> &str {
        match self {
            PackageType::Flatpak(_) => "Flatpak",
            PackageType::Snap(_) => "Snap",
            PackageType::Deb(_) => "APT / Deb",
            PackageType::Local => "Local / File",
        }
    }

    pub fn badge_css_class(&self) -> &str {
        match self {
            PackageType::Flatpak(_) => "badge-flatpak",
            PackageType::Snap(_) => "badge-snap",
            PackageType::Deb(_) => "badge-deb",
            PackageType::Local => "badge-local",
        }
    }

    pub fn details(&self) -> String {
        match self {
            PackageType::Flatpak(id) => format!("Flatpak App ID: {}", id),
            PackageType::Snap(name) => format!("Snap Package: {}", name),
            PackageType::Deb(pkg) => format!("APT Package: {}", pkg),
            PackageType::Local => "Custom desktop shortcut / file".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub comment: String,
    pub icon: String,
    pub desktop_path: PathBuf,
    pub package_type: PackageType,
}

static DPKG_DESKTOP_CACHE: OnceLock<HashMap<PathBuf, String>> = OnceLock::new();

pub fn get_dpkg_map() -> &'static HashMap<PathBuf, String> {
    DPKG_DESKTOP_CACHE.get_or_init(|| {
        let mut map = HashMap::new();
        let dpkg_info = Path::new("/var/lib/dpkg/info");
        if let Ok(entries) = fs::read_dir(dpkg_info) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("list") {
                    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let pkg_name = file_stem.split(':').next().unwrap_or(file_stem).to_string();

                    if let Ok(file) = fs::File::open(&path) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().map_while(Result::ok) {
                            if line.ends_with(".desktop") {
                                map.insert(PathBuf::from(line.trim()), pkg_name.clone());
                            }
                        }
                    }
                }
            }
        }
        map
    })
}

pub fn scan_applications() -> Vec<AppEntry> {
    let dpkg_map = get_dpkg_map();
    let mut entries = Vec::new();
    let mut seen_keys = HashSet::new();

    let mut search_dirs = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        search_dirs.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        search_dirs.push(PathBuf::from(format!("{}/.local/share/flatpak/exports/share/applications", home)));
    }
    search_dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    search_dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    search_dirs.push(PathBuf::from("/usr/share/applications"));

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        if let Ok(read_dir) = fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app) = parse_desktop_file(&path, dpkg_map) {
                        if app.name == "luninstaller" || app.name == "Application Uninstaller" || app.name == "Удаление программ" {
                            continue;
                        }

                        let key = format!("{}:{:?}", app.name, app.package_type);
                        if !seen_keys.contains(&key) {
                            seen_keys.insert(key);
                            entries.push(app);
                        }
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}

fn parse_desktop_file(path: &Path, dpkg_map: &HashMap<PathBuf, String>) -> Option<AppEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut comment = String::new();
    let mut icon = String::new();
    let mut no_display = false;
    let mut type_application = false;
    let mut x_flatpak = None;
    let mut x_snap_instance = None;
    let mut only_show_in = None;
    let mut not_show_in = None;

    let mut in_desktop_entry = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let value = v.trim();

            match key {
                "Type" => {
                    if value.eq_ignore_ascii_case("Application") {
                        type_application = true;
                    }
                }
                "Name" => {
                    if name.is_none() {
                        name = Some(value.to_string());
                    }
                }
                "Comment" => {
                    if comment.is_empty() {
                        comment = value.to_string();
                    }
                }
                "Icon" => {
                    if icon.is_empty() {
                        icon = value.to_string();
                    }
                }
                "NoDisplay" => {
                    if value.eq_ignore_ascii_case("true") {
                        no_display = true;
                    }
                }
                "OnlyShowIn" => {
                    only_show_in = Some(value.to_string());
                }
                "NotShowIn" => {
                    not_show_in = Some(value.to_string());
                }
                "X-Flatpak" => {
                    x_flatpak = Some(value.to_string());
                }
                "X-SnapInstanceName" => {
                    x_snap_instance = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    if !type_application || no_display {
        return None;
    }

    if let Some(not_in) = not_show_in {
        if not_in.split(';').any(|s| s.eq_ignore_ascii_case("GNOME")) {
            return None;
        }
    }

    if let Some(only_in) = only_show_in {
        if !only_in.split(';').any(|s| s.eq_ignore_ascii_case("GNOME")) {
            return None;
        }
    }

    let name = name?;
    if name.trim().is_empty() {
        return None;
    }

    let path_str = path.to_string_lossy();

    let package_type = if let Some(flatpak_id) = x_flatpak {
        PackageType::Flatpak(flatpak_id)
    } else if path_str.contains("flatpak") {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        PackageType::Flatpak(stem.to_string())
    } else if let Some(snap_name) = x_snap_instance {
        PackageType::Snap(snap_name)
    } else if path_str.contains("/snap/") || path_str.contains("snapd") {
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let snap_name = file_name.split('_').next().unwrap_or(file_name);
        PackageType::Snap(snap_name.replace(".desktop", ""))
    } else if path_str.starts_with("/usr/share/applications") {
        if let Some(pkg) = dpkg_map.get(path) {
            PackageType::Deb(pkg.clone())
        } else {
            PackageType::Local
        }
    } else {
        PackageType::Local
    };

    Some(AppEntry {
        name,
        comment,
        icon,
        desktop_path: path.to_path_buf(),
        package_type,
    })
}
