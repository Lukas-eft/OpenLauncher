use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::models::SteamApp;

const EXCLUDED_APP_IDS: &[&str] = &[
    "228980",
    "1070560",
    "1391110",
    "1628350",
    "1493710",
    "1420170",
    "1580130",
    "1887720",
    "2348590",
    "2805730",
    "1826330",
    "1161040",
    "2180100",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfValue {
    Str(String),
    Map(Vec<(String, VdfValue)>),
}

impl VdfValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_str()),
            Self::Map(_) => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(String, VdfValue)]> {
        match self {
            Self::Map(entries) => Some(entries.as_slice()),
            Self::Str(_) => None,
        }
    }

    pub fn get_ci(&self, key: &str) -> Option<&VdfValue> {
        self.as_map().and_then(|entries| {
            entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v)
        })
    }

    pub fn get_str_ci(&self, key: &str) -> Option<&str> {
        self.get_ci(key).and_then(Self::as_str)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SteamScanOutput {
    pub scanned_libraries: usize,
    pub apps: Vec<SteamApp>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    LBrace,
    RBrace,
    Text(String),
}

fn tokenize_vdf(input: &str) -> AppResult<Vec<Token>> {
    let clean = input.trim_start_matches('\u{FEFF}');
    let mut chars = clean.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '/' {
            chars.next();
            if chars.peek() == Some(&'/') {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            let mut word = String::from("/");
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() || next == '{' || next == '}' || next == '"' {
                    break;
                }
                word.push(next);
                chars.next();
            }
            tokens.push(Token::Text(word));
            continue;
        }

        if ch == '[' {
            chars.next();
            for c in chars.by_ref() {
                if c == ']' {
                    break;
                }
            }
            continue;
        }

        if ch == '{' {
            chars.next();
            tokens.push(Token::LBrace);
            continue;
        }

        if ch == '}' {
            chars.next();
            tokens.push(Token::RBrace);
            continue;
        }

        if ch == '"' {
            chars.next();
            let mut buf = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    match chars.next() {
                        Some('\\') => buf.push('\\'),
                        Some('"') => buf.push('"'),
                        Some('n') => buf.push('\n'),
                        Some('t') => buf.push('\t'),
                        Some('r') => buf.push('\r'),
                        Some(other) => {

                            buf.push('\\');
                            buf.push(other);
                        }
                        None => {
                            return Err(AppError::VdfParse(
                                "Unerwartetes Dateiende in Escape-Sequenz".to_string(),
                            ))
                        }
                    }
                } else {
                    buf.push(c);
                }
            }
            if !closed {
                return Err(AppError::VdfParse(
                    "Nicht geschlossene Zeichenkette im VDF/ACF-Dokument".to_string(),
                ));
            }
            tokens.push(Token::Text(buf));
            continue;
        }

        let mut word = String::new();
        while let Some(&next) = chars.peek() {
            if next.is_whitespace() || next == '{' || next == '}' || next == '"' {
                break;
            }
            word.push(next);
            chars.next();
        }
        tokens.push(Token::Text(word));
    }

    Ok(tokens)
}

fn parse_map_body(tokens: &[Token], pos: &mut usize, expect_rbrace: bool) -> AppResult<VdfValue> {
    let mut entries = Vec::new();

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::RBrace => {
                if expect_rbrace {
                    *pos += 1;
                    return Ok(VdfValue::Map(entries));
                }
                return Err(AppError::VdfParse(
                    "Unerwartete schließende Klammer '}' auf oberster Ebene".to_string(),
                ));
            }
            Token::LBrace => {
                return Err(AppError::VdfParse(
                    "Unerwartete öffnende Klammer '{' ohne Schlüssel".to_string(),
                ));
            }
            Token::Text(key) => {
                *pos += 1;
                if *pos >= tokens.len() {
                    return Err(AppError::VdfParse(format!(
                        "Fehlender Wert für Schlüssel '{key}'"
                    )));
                }
                match &tokens[*pos] {
                    Token::Text(val) => {
                        *pos += 1;
                        entries.push((key.clone(), VdfValue::Str(val.clone())));
                    }
                    Token::LBrace => {
                        *pos += 1;
                        let nested = parse_map_body(tokens, pos, true)?;
                        entries.push((key.clone(), nested));
                    }
                    Token::RBrace => {
                        return Err(AppError::VdfParse(format!(
                            "Unerwartetes '}}' nach Schlüssel '{key}'"
                        )));
                    }
                }
            }
        }
    }

    if expect_rbrace {
        return Err(AppError::VdfParse(
            "Fehlende schließende Klammer '}' im VDF/ACF-Dokument".to_string(),
        ));
    }

    Ok(VdfValue::Map(entries))
}

pub fn parse_vdf(input: &str) -> AppResult<VdfValue> {
    let tokens = tokenize_vdf(input)?;
    let mut pos = 0;
    parse_map_body(&tokens, &mut pos, false)
}

pub fn steam_cover_url(app_id: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/library_600x900_2x.jpg")
}

pub fn steam_hero_url(app_id: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/library_hero.jpg")
}

pub fn is_excluded_steam_app(app_id: &str, name: &str) -> bool {
    let id = app_id.trim();
    if EXCLUDED_APP_IDS.contains(&id) {
        return true;
    }
    let lower = name.trim().to_ascii_lowercase();
    lower == "proton"
        || lower.starts_with("proton ")
        || lower.starts_with("proton-")
        || lower.starts_with("steam linux runtime")
        || lower.starts_with("steamworks common redistributables")
}

pub fn parse_library_folders(
    vdf_content: &str,
    primary_steam_root: Option<&Path>,
) -> AppResult<Vec<PathBuf>> {
    let parsed = parse_vdf(vdf_content)?;
    let root_map = parsed.get_ci("libraryfolders").unwrap_or(&parsed);
    let entries = root_map.as_map().ok_or_else(|| {
        AppError::VdfParse("Ungültige Struktur in libraryfolders.vdf".to_string())
    })?;

    let mut libraries = Vec::new();
    let mut seen = HashSet::new();

    if let Some(primary) = primary_steam_root {
        push_unique_path(&mut libraries, &mut seen, primary.to_path_buf());
    }

    for (key, value) in entries {
        match value {

            VdfValue::Map(_) => {
                if let Some(path_str) = value.get_str_ci("path") {
                    let trimmed = path_str.trim();
                    if !trimmed.is_empty() {
                        push_unique_path(&mut libraries, &mut seen, PathBuf::from(trimmed));
                    }
                }
            }

            VdfValue::Str(path_str) if key.chars().all(|c| c.is_ascii_digit()) => {
                let trimmed = path_str.trim();
                if !trimmed.is_empty() {
                    push_unique_path(&mut libraries, &mut seen, PathBuf::from(trimmed));
                }
            }
            VdfValue::Str(_) => {}
        }
    }

    Ok(libraries)
}

pub fn parse_app_manifest(acf_content: &str, steamapps_dir: &Path) -> AppResult<SteamApp> {
    let parsed = parse_vdf(acf_content)?;
    let app_state = parsed.get_ci("AppState").unwrap_or(&parsed);

    let app_id = app_state
        .get_str_ci("appid")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::VdfParse("Fehlende 'appid' im App-Manifest".to_string()))?
        .to_string();

    if !app_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::VdfParse(format!(
            "Ungültige numerische appid '{app_id}'"
        )));
    }

    let name = app_state
        .get_str_ci("name")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::VdfParse(format!("Fehlender 'name' für AppID {app_id}")))?
        .to_string();

    if let Some(flags_str) = app_state.get_str_ci("StateFlags") {
        if let Ok(flags) = flags_str.trim().parse::<u32>() {
            if (flags & 4) == 0 {
                return Err(AppError::VdfParse(format!(
                    "AppID {app_id} is not installed (StateFlags={flags})"
                )));
            }
        }
    }

    let installdir = app_state
        .get_str_ci("installdir")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::VdfParse(format!("Fehlendes 'installdir' für AppID {app_id}")))?;

    Ok(SteamApp {
        cover_url: steam_cover_url(&app_id),
        hero_url: steam_hero_url(&app_id),
        app_id,
        name,
        install_dir: steamapps_dir.join("common").join(installdir),
    })
}

pub fn candidate_linux_steam_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ]
}

pub fn discover_steam_roots(custom_path: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    if let Some(custom) = custom_path {
        let normalized = if custom
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("steamapps"))
        {
            custom.parent().map(Path::to_path_buf).unwrap_or_else(|| custom.to_path_buf())
        } else {
            custom.to_path_buf()
        };
        push_unique_path(&mut candidates, &mut seen, normalized);
    }

    #[cfg(target_os = "windows")]
    {
        for reg_path in read_windows_steam_registry_paths() {
            push_unique_path(&mut candidates, &mut seen, reg_path);
        }
        push_unique_path(
            &mut candidates,
            &mut seen,
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
        );
        push_unique_path(
            &mut candidates,
            &mut seen,
            PathBuf::from(r"C:\Program Files\Steam"),
        );
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            for p in candidate_linux_steam_roots(&home) {
                push_unique_path(&mut candidates, &mut seen, p);
            }
        }
    }

    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

pub fn find_steam_executable(roots: &[PathBuf]) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let candidates = ["steam.exe", "Steam.exe"];
    #[cfg(not(target_os = "windows"))]
    let candidates = ["steam.sh", "steam"];

    for root in roots {
        for name in candidates {
            let exe = root.join(name);
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn read_windows_steam_registry_paths() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let mut out = Vec::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(path) = key.get_value::<String, _>("SteamPath") {
            if !path.trim().is_empty() {
                out.push(PathBuf::from(path));
            }
        }
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        r"SOFTWARE\WOW6432Node\Valve\Steam",
        r"SOFTWARE\Valve\Steam",
    ] {
        if let Ok(key) = hklm.open_subkey(subkey) {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                if !path.trim().is_empty() {
                    out.push(PathBuf::from(path));
                }
            }
        }
    }

    out
}

pub fn scan_steam_libraries(custom_path: Option<&Path>) -> SteamScanOutput {
    let roots = discover_steam_roots(custom_path);
    if roots.is_empty() {
        return SteamScanOutput {
            scanned_libraries: 0,
            apps: Vec::new(),
            warnings: vec![
                "Keine Steam-Installation gefunden. Optional kann in den Einstellungen ein benutzerdefinierter Steam-Pfad angegeben werden.".to_string(),
            ],
        };
    }
    scan_steam_roots(&roots)
}

pub fn scan_steam_roots(roots: &[PathBuf]) -> SteamScanOutput {
    let mut output = SteamScanOutput::default();
    let mut library_dirs = Vec::new();
    let mut seen_libs = HashSet::new();

    for root in roots {
        let vdf_candidates = [
            root.join("steamapps").join("libraryfolders.vdf"),
            root.join("config").join("libraryfolders.vdf"),
        ];

        if let Some(vdf_path) = vdf_candidates.iter().find(|p| p.is_file()) {
            match fs::read_to_string(vdf_path) {
                Ok(content) => match parse_library_folders(&content, Some(root)) {
                    Ok(libs) => {
                        for lib in libs {
                            push_unique_path(&mut library_dirs, &mut seen_libs, lib);
                        }
                    }
                    Err(err) => output.warnings.push(format!(
                        "{}: {err}",
                        vdf_path.display()
                    )),
                },
                Err(err) => output.warnings.push(format!(
                    "{} konnte nicht gelesen werden: {err}",
                    vdf_path.display()
                )),
            }
        } else if root.join("steamapps").is_dir() {
            push_unique_path(&mut library_dirs, &mut seen_libs, root.clone());
        } else {
            output.warnings.push(format!(
                "Kein steamapps-Verzeichnis unter {} gefunden",
                root.display()
            ));
        }
    }

    let mut seen_app_ids = HashSet::new();

    for lib_root in library_dirs {
        let steamapps_dir = lib_root.join("steamapps");
        if !steamapps_dir.is_dir() {
            continue;
        }
        output.scanned_libraries += 1;

        let entries = match fs::read_dir(&steamapps_dir) {
            Ok(e) => e,
            Err(err) => {
                output.warnings.push(format!(
                    "{} konnte nicht geöffnet werden: {err}",
                    steamapps_dir.display()
                ));
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !file_name.starts_with("appmanifest_") || !file_name.ends_with(".acf") {
                continue;
            }
            if !path.is_file() {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(err) => {
                    output.warnings.push(format!(
                        "{} konnte nicht gelesen werden: {err}",
                        path.display()
                    ));
                    continue;
                }
            };

            match parse_app_manifest(&content, &steamapps_dir) {
                Ok(mut app) => {
                    if is_excluded_steam_app(&app.app_id, &app.name) || !app.install_dir.is_dir() {
                        continue;
                    }
                    if seen_app_ids.insert(app.app_id.clone()) {
                        let (local_cover, local_hero) = find_local_steam_assets(roots, &app.app_id);
                        if let Some(cover) = local_cover {
                            app.cover_url = cover.to_string_lossy().into_owned();
                        }
                        if let Some(hero) = local_hero {
                            app.hero_url = hero.to_string_lossy().into_owned();
                        }
                        output.apps.push(app);
                    }
                }
                Err(err) => output
                    .warnings
                    .push(format!("{}: {err}", path.display())),
            }
        }
    }

    output.apps.sort_by_key(|a| a.name.to_lowercase());
    output
}

pub fn find_local_steam_assets(
    roots: &[PathBuf],
    app_id: &str,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let id = app_id.trim();
    if id.is_empty() {
        return (None, None);
    }

    for root in roots {
        let cache_root = root.join("appcache").join("librarycache");
        let app_dir = cache_root.join(id);
        if !app_dir.is_dir() {
            continue;
        }

        let mut search_dirs = vec![app_dir.clone()];
        if let Ok(entries) = fs::read_dir(&app_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    search_dirs.push(p);
                }
            }
        }

        let cover_names = [
            "library_600x900.jpg",
            "library_600x900.png",
            "library_capsule.jpg",
            "library_capsule.png",
            "library_header.jpg",
            "header.jpg",
            "header_german.jpg",
        ];
        let mut found_cover: Option<PathBuf> = None;
        'cover_loop: for name in cover_names {
            for dir in &search_dirs {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    found_cover = Some(candidate);
                    break 'cover_loop;
                }
            }
        }

        if found_cover.is_none() {
            if let Ok(entries) = fs::read_dir(&app_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_file() {
                        continue;
                    }
                    let fname = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if (fname.ends_with(".jpg") || fname.ends_with(".png"))
                        && !fname.starts_with("library_hero")
                        && !fname.starts_with("logo")
                    {
                        found_cover = Some(p);
                        break;
                    }
                }
            }
        }

        let hero_names = [
            "library_hero.jpg",
            "library_hero.png",
            "library_header.jpg",
            "header.jpg",
        ];
        let mut found_hero: Option<PathBuf> = None;
        'hero_loop: for name in hero_names {
            for dir in &search_dirs {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    found_hero = Some(candidate);
                    break 'hero_loop;
                }
            }
        }

        if found_cover.is_some() || found_hero.is_some() {
            return (found_cover, found_hero);
        }
    }

    (None, None)
}

fn push_unique_path(out: &mut Vec<PathBuf>, seen: &mut HashSet<String>, path: PathBuf) {
    let key = normalize_path_key(&path);
    if !key.is_empty() && seen.insert(key) {
        out.push(path);
    }
}

fn normalize_path_key(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim().trim_end_matches(['/', '\\']);
    #[cfg(target_os = "windows")]
    {
        trimmed.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_parse_vdf_nested_and_escaped_paths() {
        let sample = r#"
        // Kommentarzeile
        "libraryfolders"
        {
            "0"
            {
                "path"      "C:\\Program Files (x86)\\Steam"
                "label"     ""
                "apps"
                {
                    "1145360"   "12345678"
                }
            }
        }
        "#;

        let vdf = parse_vdf(sample).expect("parse vdf");
        let lf = vdf.get_ci("libraryfolders").expect("libraryfolders");
        let zero = lf.get_ci("0").expect("entry 0");
        assert_eq!(
            zero.get_str_ci("path"),
            Some(r"C:\Program Files (x86)\Steam")
        );
        let apps = zero.get_ci("apps").expect("apps");
        assert_eq!(apps.get_str_ci("1145360"), Some("12345678"));
    }

    #[test]
    fn test_parse_vdf_malformed_returns_error() {
        assert!(parse_vdf("\"root\" { \"k\" \"unterminated").is_err());
        assert!(parse_vdf("\"root\" { \"k\" \"v\"").is_err());
        assert!(parse_vdf("\"root\" }").is_err());
    }

    #[test]
    fn test_parse_library_folders_modern_and_legacy() {
        let modern = r#"
        "libraryfolders"
        {
            "0"
            {
                "path" "C:\\Program Files (x86)\\Steam"
            }
            "1"
            {
                "path" "D:\\SteamLibrary"
            }
        }
        "#;
        let libs = parse_library_folders(modern, None).expect("modern libs");
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[0], PathBuf::from(r"C:\Program Files (x86)\Steam"));
        assert_eq!(libs[1], PathBuf::from(r"D:\SteamLibrary"));

        let legacy = r#"
        "LibraryFolders"
        {
            "TimeNextStatsReport" "1600000000"
            "1" "E:\\Games\\Steam"
        }
        "#;
        let legacy_libs =
            parse_library_folders(legacy, Some(Path::new("/home/user/.local/share/Steam")))
                .expect("legacy libs");
        assert_eq!(legacy_libs.len(), 2);
        assert_eq!(legacy_libs[1], PathBuf::from(r"E:\Games\Steam"));
    }

    #[test]
    fn test_parse_app_manifest_extracts_fields_and_cover_url() {
        let acf = r#"
        "AppState"
        {
            "appid"         "1245620"
            "Universe"      "1"
            "name"          "ELDEN RING"
            "StateFlags"    "4"
            "installdir"    "ELDEN RING"
        }
        "#;
        let steamapps = Path::new("/mnt/games/SteamLibrary/steamapps");
        let app = parse_app_manifest(acf, steamapps).expect("parse manifest");

        assert_eq!(app.app_id, "1245620");
        assert_eq!(app.name, "ELDEN RING");
        assert_eq!(app.install_dir, steamapps.join("common").join("ELDEN RING"));
        assert_eq!(
            app.cover_url,
            "https://cdn.cloudflare.steamstatic.com/steam/apps/1245620/library_600x900_2x.jpg"
        );
        assert_eq!(
            app.hero_url,
            "https://cdn.cloudflare.steamstatic.com/steam/apps/1245620/library_hero.jpg"
        );
    }

    #[test]
    fn test_excluded_steam_tools_filtered() {
        assert!(is_excluded_steam_app(
            "228980",
            "Steamworks Common Redistributables"
        ));
        assert!(is_excluded_steam_app("1493710", "Proton Experimental"));
        assert!(is_excluded_steam_app("9999999", "Proton 10.0"));
        assert!(is_excluded_steam_app(
            "1628350",
            "Steam Linux Runtime 3.0 (sniper)"
        ));
        assert!(!is_excluded_steam_app("1145360", "Hades"));
    }

    #[test]
    fn test_scan_steam_roots_end_to_end_tempdir() {
        let tmp = TempDir::new().expect("tempdir");
        let primary_root = tmp.path().join("SteamPrimary");
        let secondary_root = tmp.path().join("SteamLibrary2");

        fs::create_dir_all(primary_root.join("steamapps").join("common").join("Hades"))
            .expect("create primary");
        fs::create_dir_all(
            secondary_root
                .join("steamapps")
                .join("common")
                .join("Celeste"),
        )
        .expect("create secondary");

        let p0 = primary_root.to_string_lossy().replace('\\', "\\\\");
        let p1 = secondary_root.to_string_lossy().replace('\\', "\\\\");
        let libraryfolders_vdf = format!(
            "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{p0}\" }}\n  \"1\" {{ \"path\" \"{p1}\" }}\n}}\n"
        );
        fs::write(
            primary_root.join("steamapps").join("libraryfolders.vdf"),
            libraryfolders_vdf,
        )
        .expect("write vdf");

        fs::write(
            primary_root.join("steamapps").join("appmanifest_1145360.acf"),
            r#""AppState" { "appid" "1145360" "name" "Hades" "installdir" "Hades" }"#,
        )
        .expect("write hades");
        fs::write(
            primary_root.join("steamapps").join("appmanifest_1493710.acf"),
            r#""AppState" { "appid" "1493710" "name" "Proton Experimental" "installdir" "Proton - Experimental" }"#,
        )
        .expect("write proton");
        fs::write(
            secondary_root
                .join("steamapps")
                .join("appmanifest_504230.acf"),
            r#""AppState" { "appid" "504230" "name" "Celeste" "installdir" "Celeste" }"#,
        )
        .expect("write celeste");

        let scan = scan_steam_roots(&[primary_root]);
        assert_eq!(scan.scanned_libraries, 2);
        assert!(scan.warnings.is_empty(), "warnings: {:?}", scan.warnings);
        assert_eq!(scan.apps.len(), 2);
        assert_eq!(scan.apps[0].name, "Celeste");
        assert_eq!(scan.apps[0].app_id, "504230");
        assert_eq!(scan.apps[1].name, "Hades");
        assert_eq!(scan.apps[1].app_id, "1145360");
    }

    #[test]
    fn test_candidate_linux_steam_roots() {
        let roots = candidate_linux_steam_roots(Path::new("/home/player"));
        assert!(roots.contains(&PathBuf::from("/home/player/.local/share/Steam")));
        assert!(roots.contains(&PathBuf::from("/home/player/.var/app/com.valvesoftware.Steam/.local/share/Steam")));
    }

    #[test]
    fn test_find_steam_executable_tempdir() {
        let tmp = TempDir::new().expect("tempdir");
        let empty_root = tmp.path().join("EmptySteam");
        let valid_root = tmp.path().join("ValidSteam");
        fs::create_dir_all(&empty_root).expect("create empty");
        fs::create_dir_all(&valid_root).expect("create valid");

        #[cfg(target_os = "windows")]
        let expected = valid_root.join("steam.exe");
        #[cfg(not(target_os = "windows"))]
        let expected = valid_root.join("steam");

        fs::write(&expected, b"dummy").expect("write dummy steam exe");

        let found = find_steam_executable(&[empty_root, valid_root]);
        assert_eq!(found.as_ref(), Some(&expected));
    }
}
