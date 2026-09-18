use crate::app_scanner::{AppEntry, PackageType};
use std::fs;
use std::process::Command;

pub struct UninstallResult {
    pub success: bool,
    pub message: String,
}

pub fn uninstall_app(app: &AppEntry) -> UninstallResult {
    match &app.package_type {
        PackageType::Flatpak(id) => {
            let status = Command::new("flatpak")
                .args(["uninstall", "-y", id])
                .status();

            match status {
                Ok(s) if s.success() => UninstallResult {
                    success: true,
                    message: format!("'{}' was successfully uninstalled via Flatpak.", app.name),
                },
                Ok(s) => UninstallResult {
                    success: false,
                    message: format!("Flatpak returned an error (exit code: {}).", s),
                },
                Err(e) => UninstallResult {
                    success: false,
                    message: format!("Failed to run 'flatpak' command: {}", e),
                },
            }
        }

        PackageType::Snap(name) => {
            let status = Command::new("pkexec")
                .args(["snap", "remove", name])
                .status();

            match status {
                Ok(s) if s.success() => UninstallResult {
                    success: true,
                    message: format!("Snap package '{}' was successfully removed.", app.name),
                },
                Ok(s) => UninstallResult {
                    success: false,
                    message: format!("Snap removal failed (exit code: {}). Authorization might have been canceled.", s),
                },
                Err(e) => UninstallResult {
                    success: false,
                    message: format!("Failed to launch pkexec: {}", e),
                },
            }
        }

        PackageType::Deb(pkg) => {
            let status = Command::new("pkexec")
                .args(["apt-get", "purge", "-y", pkg])
                .status();

            match status {
                Ok(s) if s.success() => UninstallResult {
                    success: true,
                    message: format!("System package '{}' ({}) was successfully purged.", app.name, pkg),
                },
                Ok(s) => UninstallResult {
                    success: false,
                    message: format!("APT removal failed (exit code: {}). Authorization might have been canceled.", s),
                },
                Err(e) => UninstallResult {
                    success: false,
                    message: format!("Failed to run pkexec apt-get: {}", e),
                },
            }
        }

        PackageType::Local => {
            let path = &app.desktop_path;
            if path.exists() {
                if let Ok(home) = std::env::var("HOME") {
                    if path.starts_with(&home) {
                        if let Err(e) = fs::remove_file(path) {
                            return UninstallResult {
                                success: false,
                                message: format!("Failed to delete file {}: {}", path.display(), e),
                            };
                        }
                        return UninstallResult {
                            success: true,
                            message: format!("Shortcut for '{}' has been removed.", app.name),
                        };
                    }
                }

                let status = Command::new("pkexec")
                    .arg("rm")
                    .arg("-f")
                    .arg(path)
                    .status();

                match status {
                    Ok(s) if s.success() => UninstallResult {
                        success: true,
                        message: format!("File '{}' has been removed.", path.display()),
                    },
                    Ok(s) => UninstallResult {
                        success: false,
                        message: format!("Failed to delete file: {}", s),
                    },
                    Err(e) => UninstallResult {
                        success: false,
                        message: format!("Failed to launch pkexec: {}", e),
                    },
                }
            } else {
                UninstallResult {
                    success: true,
                    message: "File does not exist anymore.".to_string(),
                }
            }
        }
    }
}
