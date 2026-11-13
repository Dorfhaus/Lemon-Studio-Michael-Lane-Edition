// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Where ML Studio keeps its own files: settings, the remembered theme and design, presets, browser folders.
//!
//! One folder per user, shared by the application and its own plugins (a plugin runs in its own module and reads the
//! theme from here itself):
//!
//! - Windows: `%APPDATA%\ML Studio`
//! - macOS: `~/Library/Application Support/ML Studio`
//! - Linux and other Unix: `$XDG_CONFIG_HOME/ML Studio`, else `~/.config/ML Studio`
//!
//! `OPENSTUDIO_USER_DIR` overrides the folder on every platform. Targets without a file system (the web) have none.

use std::path::PathBuf;

/// Current names for new profiles. Existing paths are kept so absolute sample and preset links still work.
pub const APP_NAME: &str = "ML Studio";
pub const PREVIEW_NAME: &str = "ML Studio Public Preview";
const LEGACY_APP: &str = "Lemon Studio";
const LEGACY_PREVIEW: &str = "Lemon Studio Public Preview";

/// The active profile, with the launcher's explicit choice taking precedence.
pub fn user_dir() -> Option<PathBuf> {
    if let Some(dir) = var("OPENSTUDIO_USER_DIR") {
        return Some(dir);
    }
    let root = platform_dir()?;
    Some(profile_dir(&root, development_exe(std::env::current_exe().ok().as_deref())))
}

fn profile_dir(root: &std::path::Path, development: bool) -> PathBuf {
    let (current, legacy) = if development { (PREVIEW_NAME, LEGACY_PREVIEW) } else { (APP_NAME, LEGACY_APP) };
    let current = root.join(current);
    let legacy = root.join(legacy);
    if current.exists() || !legacy.is_dir() { current } else { legacy }
}

/// The original OpenStudio profile, migrated by the main app on first launch.
pub fn legacy_dir() -> Option<PathBuf> {
    if var("OPENSTUDIO_USER_DIR").is_some() || development_exe(std::env::current_exe().ok().as_deref()) {
        return None;
    }
    platform_dir().map(|dir| dir.join("OpenStudio"))
}

pub fn is_preview_profile(path: &std::path::Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| matches!(name, PREVIEW_NAME | LEGACY_PREVIEW))
}

fn development_exe(exe: Option<&std::path::Path>) -> bool {
    exe.and_then(std::path::Path::file_stem).and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("lemonstudio-development"))
}

/// A file or folder inside [`user_dir`].
pub fn user_file(name: &str) -> Option<PathBuf> {
    user_dir().map(|d| d.join(name))
}

/// A set, absolute environment variable (empty and relative values are ignored, as XDG asks).
fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

#[cfg(windows)]
fn platform_dir() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{System::Com::CoTaskMemFree, UI::Shell::{SHGetKnownFolderPath, FOLDERID_RoamingAppData}};
    // Ask Windows for this user's actual folder, independently of the launcher's environment.
    let mut path = std::ptr::null_mut();
    // SAFETY: Windows allocates the terminated UTF-16 result; it is copied before being freed.
    let resolved = unsafe {
        let result = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, 0, std::ptr::null_mut(), &mut path);
        if result < 0 || path.is_null() {
            None
        } else {
            let mut len = 0;
            while *path.add(len) != 0 { len += 1; }
            let folder = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(path, len)));
            CoTaskMemFree(path.cast());
            Some(folder)
        }
    };
    resolved.or_else(|| var("APPDATA"))
}

#[cfg(target_os = "macos")]
fn platform_dir() -> Option<PathBuf> {
    var("HOME").map(|h| h.join("Library").join("Application Support"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_dir() -> Option<PathBuf> {
    var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))
}

#[cfg(not(any(windows, unix)))]
fn platform_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn development_profile_is_isolated_for_direct_launch() {
        assert!(development_exe(Some(Path::new("lemonstudio-development.exe"))));
        assert!(development_exe(Some(Path::new("LEMONSTUDIO-DEVELOPMENT.exe"))));
        assert!(!development_exe(Some(Path::new("lemonstudio.exe"))));
        assert!(!development_exe(None));
    }

    #[test]
    fn new_profiles_use_ml_brand_and_existing_profiles_keep_their_paths() {
        let root = std::env::temp_dir().join(format!("ml-studio-profile-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        assert_eq!(profile_dir(&root, false), root.join(APP_NAME));
        assert_eq!(profile_dir(&root, true), root.join(PREVIEW_NAME));
        let legacy = root.join(LEGACY_APP);
        let preview = root.join(LEGACY_PREVIEW);
        std::fs::create_dir(&legacy).unwrap();
        assert_eq!(profile_dir(&root, false), legacy);
        assert_eq!(profile_dir(&root, true), root.join(PREVIEW_NAME));
        std::fs::create_dir(&preview).unwrap();
        assert_eq!(profile_dir(&root, true), preview);
        assert!(is_preview_profile(&preview));
        assert!(is_preview_profile(&root.join(PREVIEW_NAME)));
        assert!(!is_preview_profile(&legacy));
        let current = root.join(APP_NAME);
        std::fs::create_dir(&current).unwrap();
        assert_eq!(profile_dir(&root, false), current);
        for dir in [current, preview, legacy, root] {
            std::fs::remove_dir(dir).unwrap();
        }
    }
}
