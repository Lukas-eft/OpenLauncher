use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::steam;

const MAX_COVER_BYTES: u64 = 15 * 1024 * 1024;
const ALLOWED_IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "avif"];

type SteamAssetMap = HashMap<String, (Option<String>, Option<String>)>;

static STEAM_ASSET_CACHE: OnceLock<Mutex<SteamAssetMap>> = OnceLock::new();

/// Resolves the OpenLauncher application data directory (`%APPDATA%\com.openlauncher.app` on Windows),
/// creates it if needed, and migrates any legacy `openlibrary.sqlite`.
pub fn resolve_app_data_dir() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        AppError::Internal("Could not resolve AppData directory".to_string())
    })?;
    let app_data_dir = base.join("com.openlauncher.app");
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("openlauncher.sqlite");
    let legacy_db_path = app_data_dir.join("openlibrary.sqlite");
    if !db_path.exists() && legacy_db_path.is_file() {
        let _ = fs::copy(&legacy_db_path, &db_path);
    }

    fs::create_dir_all(app_data_dir.join("covers"))?;
    Ok(app_data_dir)
}

/// Copies local cover files into the managed `covers/` directory if they reside outside it.
pub fn ensure_managed_cover(
    cover_url: &mut Option<String>,
    cover_is_local: &mut bool,
    covers_dir: &Path,
) {
    if !*cover_is_local {
        return;
    }
    let Some(ref raw_cover) = *cover_url else {
        return;
    };
    let cover_path = Path::new(raw_cover.trim());
    if !is_inside_dir(cover_path, covers_dir) {
        match copy_cover_to_dir(cover_path, covers_dir) {
            Ok(copied) => *cover_url = Some(copied),
            Err(_) => {
                *cover_url = None;
                *cover_is_local = false;
            }
        }
    }
}

/// Validates and copies a selected cover image into the managed `covers/` directory.
pub fn copy_cover_to_dir(src: &Path, covers_dir: &Path) -> AppResult<String> {
    if !src.is_file() {
        return Err(AppError::InvalidInput(
            "Selected image file does not exist".to_string(),
        ));
    }

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| ALLOWED_IMAGE_EXTS.contains(&e.as_str()))
        .ok_or_else(|| {
            AppError::InvalidInput(
                "Supported cover formats: PNG, JPG, JPEG, WEBP, GIF, AVIF".to_string(),
            )
        })?;

    let meta = fs::metadata(src)?;
    if meta.len() > MAX_COVER_BYTES {
        return Err(AppError::InvalidInput(
            "Cover image must be at most 15 MB".to_string(),
        ));
    }

    fs::create_dir_all(covers_dir)?;
    let target_name = format!("{}.{ext}", Uuid::new_v4());
    let target_path = covers_dir.join(target_name);
    fs::copy(src, &target_path)?;

    Ok(target_path.to_string_lossy().into_owned())
}

/// Removes a managed local cover file when a game is deleted.
pub fn remove_cover_if_managed(cover_path: &Path, covers_dir: &Path) {
    if let (Ok(canon_cover), Ok(canon_dir)) =
        (fs::canonicalize(cover_path), fs::canonicalize(covers_dir))
    {
        if canon_cover.starts_with(&canon_dir) && canon_cover.is_file() {
            let _ = fs::remove_file(canon_cover);
        }
    }
}

fn is_inside_dir(path: &Path, dir: &Path) -> bool {
    if let (Ok(canon_path), Ok(canon_dir)) = (fs::canonicalize(path), fs::canonicalize(dir)) {
        canon_path.starts_with(canon_dir)
    } else {
        false
    }
}

/// Resolves an image URI suitable for `egui_extras` (`file:///...` or `https://...`).
///
/// Automatically resolves Steam CDN URLs to local `Steam/appcache/librarycache/<appid>` files
/// if available on disk so covers load even when Valve's legacy CDN URL returns 404.
pub fn cover_image_uri(cover_url: Option<&str>, cover_is_local: bool) -> Option<String> {
    let trimmed = cover_url?.trim();
    if trimmed.is_empty() {
        return None;
    }

    if cover_is_local {
        return Some(local_path_to_file_uri(trimmed));
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Some(app_id) = extract_steam_app_id_from_url(trimmed) {
            let is_hero = trimmed.contains("library_hero");
            if let Some(local_file) = lookup_cached_steam_asset(app_id, is_hero) {
                return Some(local_path_to_file_uri(&local_file));
            }
        }
        return Some(trimmed.to_string());
    }

    if trimmed.starts_with("file://") {
        return Some(trimmed.to_string());
    }

    Some(local_path_to_file_uri(trimmed))
}

fn extract_steam_app_id_from_url(url: &str) -> Option<&str> {
    let marker = "/steam/apps/";
    let idx = url.find(marker)?;
    let rest = &url[idx + marker.len()..];
    let end = rest.find('/')?;
    let id = &rest[..end];
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Some(id)
    } else {
        None
    }
}

fn lookup_cached_steam_asset(app_id: &str, is_hero: bool) -> Option<String> {
    let cache = STEAM_ASSET_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().ok()?;
    let (cover, hero) = guard
        .entry(app_id.to_string())
        .or_insert_with(|| {
            let roots = steam::discover_steam_roots(None);
            let (c, h) = steam::find_local_steam_assets(&roots, app_id);
            (
                c.map(|p| p.to_string_lossy().into_owned()),
                h.map(|p| p.to_string_lossy().into_owned()),
            )
        })
        .clone();

    if is_hero {
        hero.or(cover)
    } else {
        cover
    }
}

fn local_path_to_file_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("file://") {
        normalized
    } else if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_cover_image_uri_handles_local_spaces_and_https() {
        assert_eq!(cover_image_uri(None, false), None);
        assert_eq!(cover_image_uri(Some("   "), true), None);

        assert_eq!(
            cover_image_uri(
                Some(r"C:\Users\Lukas\AppData\Roaming\com.openlauncher.app\covers\my cover.png"),
                true
            )
            .as_deref(),
            Some("file:///C:/Users/Lukas/AppData/Roaming/com.openlauncher.app/covers/my cover.png")
        );

        assert_eq!(
            cover_image_uri(Some("/home/user/covers/zelda.webp"), true).as_deref(),
            Some("file:///home/user/covers/zelda.webp")
        );

        assert_eq!(
            cover_image_uri(
                Some("https://cdn.cloudflare.steamstatic.com/steam/apps/999999999/library_600x900_2x.jpg"),
                false
            )
            .as_deref(),
            Some("https://cdn.cloudflare.steamstatic.com/steam/apps/999999999/library_600x900_2x.jpg")
        );
    }

    #[test]
    fn test_copy_cover_to_dir_and_remove_managed() {
        let tmp = TempDir::new().expect("tempdir");
        let src_dir = tmp.path().join("src");
        let covers_dir = tmp.path().join("covers");
        fs::create_dir_all(&src_dir).expect("mkdir src");

        let src_png = src_dir.join("boxart.PNG");
        fs::write(&src_png, b"fake-png-bytes").expect("write png");

        let copied = copy_cover_to_dir(&src_png, &covers_dir).expect("copy png");
        let copied_path = PathBuf::from(&copied);
        assert!(copied_path.is_file());
        assert!(copied_path.starts_with(&covers_dir));
        assert_eq!(copied_path.extension().and_then(|e| e.to_str()), Some("png"));

        // Invalid extension rejected
        let bad = src_dir.join("readme.txt");
        fs::write(&bad, b"text").expect("write txt");
        assert!(matches!(
            copy_cover_to_dir(&bad, &covers_dir),
            Err(AppError::InvalidInput(_))
        ));

        // Cleanup only deletes files inside covers_dir
        remove_cover_if_managed(&src_png, &covers_dir);
        assert!(src_png.is_file());
        remove_cover_if_managed(&copied_path, &covers_dir);
        assert!(!copied_path.is_file());
    }
}
