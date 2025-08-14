use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::AppHandle;

const STORAGE_DIR: &str = ".local/share/axec/appimages";
const APPLICATIONS_DIR: &str = ".local/share/applications";

fn in_flatpak_sandbox() -> bool {
    std::env::var("FLATPAK_ID").is_ok() || std::env::var("container").map(|v| v == "flatpak").unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon_path: Option<String>,
    pub desktop_file: String,
}

fn ensure_dirs() -> io::Result<(PathBuf, PathBuf)> {
    // When sandboxed, prefer XDG data dir; avoid writing system applications outside sandbox
    let data_dir = dirs::data_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG data dir not found"))?;
    let storage = data_dir.join("axec/appimages");
    let apps = if in_flatpak_sandbox() {
        // inside Flatpak, write desktop files under XDG data dir; they will only be visible to the sandbox
        data_dir.join("applications")
    } else {
        dirs::home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME not found"))?.join(APPLICATIONS_DIR)
    };
    fs::create_dir_all(&storage)?;
    fs::create_dir_all(&apps)?;
    Ok((storage, apps))
}

fn sanitize_filename(name: &str) -> String {
    let filtered: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    filtered.trim_matches('-').to_lowercase()
}

fn parse_appimage_name(path: &Path) -> String {
    let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("appimage");
    // Remove common suffixes like ".x86_64" or version numbers
    let mut base = fname.replace(
        |c: char| !(c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_'),
        " ",
    );
    base = base.split_whitespace().collect::<Vec<_>>().join(" ");
    base.trim().to_string()
}

fn write_desktop_file(name: &str, exec_path: &Path, icon_path: Option<&Path>, desktop_path: &Path) -> io::Result<()> {
    let exec_str = exec_path.to_string_lossy();
    let icon_line = icon_path.map(|p| format!("Icon={}", p.to_string_lossy())).unwrap_or_default();
    let content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{exec}\" %U\nTerminal=false\nCategories=Utility;\n{icon}\nX-AppImage-Version=1\nX-AppImage-Integrate=false\n",
        name = name,
        exec = exec_str,
        icon = icon_line
    );
    let mut f = fs::File::create(desktop_path)?;
    f.write_all(content.as_bytes())
}

fn make_executable(path: &Path) -> io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

fn extract_icon(appimage_path: &Path, target_dir: &Path, base_id: &str) -> Option<PathBuf> {
    // Try --appimage-extract . Try to locate .DirIcon or usr/share/icons
    // Fallback: None
    let tmp_dir = tempfile::Builder::new().prefix("axec-extract-").tempdir().ok()?;
    let status = Command::new(appimage_path)
        .arg("--appimage-extract")
        .current_dir(tmp_dir.path())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let squash_root = tmp_dir.path().join("squashfs-root");
    let mut candidates: Vec<PathBuf> = vec![squash_root.join(".DirIcon")];
    for sub in [
        "usr/share/icons/hicolor/256x256/apps",
        "usr/share/icons/hicolor/128x128/apps",
        "usr/share/icons/hicolor/64x64/apps",
        "usr/share/pixmaps",
    ] {
        let dir = squash_root.join(sub);
        if dir.is_dir() {
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if matches!(ext.to_ascii_lowercase().as_str(), "png" | "svg" | "xpm" | "ico") {
                            candidates.push(p);
                        }
                    }
                }
            }
        }
    }
    // Pick first existing candidate
    let icon_src = candidates.into_iter().find(|p| p.exists())?;
    let ext = icon_src
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());
    let icon_dest = target_dir.join(format!("{base_id}.{ext}"));
    if fs::copy(&icon_src, &icon_dest).is_ok() {
        Some(icon_dest)
    } else {
        None
    }
}

#[tauri::command]
fn list_apps() -> Result<Vec<AppImageEntry>, String> {
    let (storage, apps_dir) = ensure_dirs().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    if let Ok(rd) = fs::read_dir(&storage) {
        for entry in rd.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                let ext_l = ext.to_ascii_lowercase();
                if ext_l == "appimage" {
                    let name = parse_appimage_name(&p);
                    let id = sanitize_filename(&name);
                    let desktop_file = apps_dir.join(format!("axec-{id}.desktop"));
                    // find icon with id.* in storage
                    let icon_path = ["png", "svg", "ico", "xpm"].iter().map(|e| storage.join(format!("{id}.{e}"))).find(|x| x.exists());
                    result.push(AppImageEntry {
                        id: id.clone(),
                        name,
                        path: p.to_string_lossy().to_string(),
                        icon_path: icon_path.map(|p| p.to_string_lossy().to_string()),
                        desktop_file: desktop_file.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    Ok(result)
}

#[tauri::command]
fn add_appimage(file_path: String) -> Result<AppImageEntry, String> {
    let src = PathBuf::from(&file_path);
    if !src.exists() {
        return Err("File not found".into());
    }
    let (storage, apps_dir) = ensure_dirs().map_err(|e| e.to_string())?;
    let name = parse_appimage_name(&src);
    let id = sanitize_filename(&name);
    let dest_path = storage.join(format!("{id}.AppImage"));
    fs::copy(&src, &dest_path).map_err(|e| e.to_string())?;
    make_executable(&dest_path).map_err(|e| e.to_string())?;

    // Try extract icon to storage
    let icon_path = extract_icon(&dest_path, &storage, &id);

    // Create desktop file
    // Only write desktop entry outside sandbox; inside sandbox it won't be picked by host menu.
    if !in_flatpak_sandbox() {
        let desktop_path = apps_dir.join(format!("axec-{id}.desktop"));
        write_desktop_file(&name, &dest_path, icon_path.as_deref(), &desktop_path).map_err(|e| e.to_string())?;
    }

    Ok(AppImageEntry {
        id: id.clone(),
        name,
        path: dest_path.to_string_lossy().to_string(),
        icon_path: icon_path.map(|p| p.to_string_lossy().to_string()),
        desktop_file: desktop_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn remove_app(id: String) -> Result<(), String> {
    let (storage, apps_dir) = ensure_dirs().map_err(|e| e.to_string())?;
    // Remove appimage
    let mut ok_any = false;
    for ext in ["AppImage", "appimage"] {
        let p = storage.join(format!("{id}.{ext}"));
        if p.exists() {
            fs::remove_file(&p).map_err(|e| e.to_string())?;
            ok_any = true;
        }
    }
    // Remove icon variants
    for ext in ["png", "svg", "ico", "xpm"] {
        let p = storage.join(format!("{id}.{ext}"));
        let _ = fs::remove_file(p);
    }
    // Remove desktop file
    if !in_flatpak_sandbox() {
        let desktop = apps_dir.join(format!("axec-{id}.desktop"));
        if desktop.exists() {
            let _ = fs::remove_file(desktop);
            ok_any = true;
        }
    }
    if ok_any { Ok(()) } else { Err("App not found".into()) }
}

#[tauri::command]
fn launch_app(id: String) -> Result<(), String> {
    let (storage, _apps_dir) = ensure_dirs().map_err(|e| e.to_string())?;
    let app_path = ["AppImage", "appimage"].into_iter().map(|e| storage.join(format!("{id}.{e}"))).find(|p| p.exists()).ok_or("AppImage not found")?;
    Command::new(app_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![list_apps, add_appimage, remove_app, launch_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
