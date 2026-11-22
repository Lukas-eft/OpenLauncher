use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::db::ExistingGameKeys;
use crate::error::{AppError, AppResult};
use crate::launcher::parse_desktop_entry;
use crate::models::{
    DiscoveredGame, EmulatorPresetInfo, GameFileInspection, Platform,
};
use crate::steam;

const COVER_BASE_NAMES: &[&str] = &["cover", "boxart", "folder", "poster", "icon"];
const COVER_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "avif"];

const JUNK_EXE_PREFIXES: &[&str] = &[
    "unins",
    "uninstall",
    "setup",
    "install",
    "crashreport",
    "crashhandler",
    "unitycrashhandler",
    "ue4prereq",
    "ueprereq",
    "vc_redist",
    "vcredist",
    "dxsetup",
    "directx",
    "dotnet",
    "easyanticheat",
    "battleye",
    "redist",
    "activation",
    "cleanup",
    "touchup",
    "reporter",
];

const JUNK_EXE_SUBSTRINGS: &[&str] = &[
    "unins000",
    "uninstall",
    "crashreport",
    "crashhandler",
    "prereq",
    "vcredist",
    "dxwebsetup",
    "easyanticheat",
    "battleye",
    "vc_redist",
    "webview2",
];

const NON_ROM_EXTS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bat", "cmd", "sh", "txt", "md", "xml", "json", "ini", "cfg",
    "conf", "log", "png", "jpg", "jpeg", "webp", "gif", "bmp", "tga", "svg", "sav", "srm",
    "state", "db", "sqlite", "url", "lnk", "pdf", "html", "css",
];

struct EmulatorDef {
    id: &'static str,
    name: &'static str,
    system: &'static str,
    exe_names: &'static [&'static str],
    extensions: &'static [&'static str],
    default_args: &'static str,
    fullscreen_args: &'static str,
    tags: &'static [&'static str],
}

const EMULATOR_DEFS: &[EmulatorDef] = &[
    EmulatorDef {
        id: "cemu",
        name: "Cemu",
        system: "Wii U",
        exe_names: &["Cemu.exe", "cemu.exe", "cemu", "Cemu"],
        extensions: &["wua", "wux", "wud", "rpx"],
        default_args: "-g {rom}",
        fullscreen_args: "-f -g {rom}",
        tags: &["Cemu", "Wii U"],
    },
    EmulatorDef {
        id: "dolphin",
        name: "Dolphin",
        system: "GameCube / Wii",
        exe_names: &["Dolphin.exe", "dolphin-emu", "dolphin-emu-qt2"],
        extensions: &["rvz", "iso", "gcz", "wbfs", "gcm", "wad", "ciso", "dol"],
        default_args: "-b -e {rom}",
        fullscreen_args: "-b -e {rom}",
        tags: &["Dolphin", "GameCube / Wii"],
    },
    EmulatorDef {
        id: "rpcs3",
        name: "RPCS3",
        system: "PlayStation 3",
        exe_names: &["rpcs3.exe", "rpcs3"],
        extensions: &["bin", "pkg"],
        default_args: "{rom}",
        fullscreen_args: "--no-gui {rom}",
        tags: &["RPCS3", "PS3"],
    },
    EmulatorDef {
        id: "pcsx2",
        name: "PCSX2",
        system: "PlayStation 2",
        exe_names: &["pcsx2-qt.exe", "pcsx2.exe", "pcsx2-qt", "pcsx2"],
        extensions: &["chd", "iso", "cso", "gz"],
        default_args: "-batch -- {rom}",
        fullscreen_args: "-batch -fullscreen -- {rom}",
        tags: &["PCSX2", "PS2"],
    },
    EmulatorDef {
        id: "ryujinx",
        name: "Ryujinx",
        system: "Nintendo Switch",
        exe_names: &["Ryujinx.exe", "Ryujinx", "ryujinx"],
        extensions: &["nsp", "xci", "nca", "nro"],
        default_args: "{rom}",
        fullscreen_args: "--fullscreen {rom}",
        tags: &["Ryujinx", "Switch"],
    },
    EmulatorDef {
        id: "yuzu",
        name: "Yuzu / Eden",
        system: "Nintendo Switch",
        exe_names: &[
            "eden.exe",
            "sudachi.exe",
            "citron.exe",
            "yuzu.exe",
            "eden",
            "sudachi",
            "citron",
            "yuzu",
        ],
        extensions: &["nsp", "xci", "nca", "nro"],
        default_args: "-g {rom}",
        fullscreen_args: "-f -g {rom}",
        tags: &["Switch"],
    },
    EmulatorDef {
        id: "ppsspp",
        name: "PPSSPP",
        system: "PSP",
        exe_names: &[
            "PPSSPPWindows64.exe",
            "PPSSPPWindows.exe",
            "PPSSPPQt",
            "PPSSPPSDL",
        ],
        extensions: &["iso", "cso", "pbp", "chd"],
        default_args: "{rom}",
        fullscreen_args: "--fullscreen {rom}",
        tags: &["PPSSPP", "PSP"],
    },
    EmulatorDef {
        id: "duckstation",
        name: "DuckStation",
        system: "PlayStation 1",
        exe_names: &[
            "duckstation-qt-x64-ReleaseLTCG.exe",
            "duckstation-qt.exe",
            "duckstation-qt",
        ],
        extensions: &["chd", "cue", "pbp", "m3u", "iso"],
        default_args: "-batch -- {rom}",
        fullscreen_args: "-batch -fullscreen -- {rom}",
        tags: &["DuckStation", "PS1"],
    },
    EmulatorDef {
        id: "citra",
        name: "Citra / Azahar",
        system: "Nintendo 3DS",
        exe_names: &[
            "azahar.exe",
            "citra-qt.exe",
            "lime3ds.exe",
            "azahar",
            "citra-qt",
        ],
        extensions: &["3ds", "cci", "cia", "cxi"],
        default_args: "{rom}",
        fullscreen_args: "-f {rom}",
        tags: &["3DS"],
    },
    EmulatorDef {
        id: "mgba",
        name: "mGBA",
        system: "Game Boy / GBA",
        exe_names: &["mGBA.exe", "mgba-qt.exe", "mgba-qt", "mgba"],
        extensions: &["gba", "gbc", "gb"],
        default_args: "{rom}",
        fullscreen_args: "-f {rom}",
        tags: &["mGBA", "Game Boy"],
    },
    EmulatorDef {
        id: "custom",
        name: "Custom Emulator",
        system: "Custom",
        exe_names: &[],
        extensions: &[],
        default_args: "{rom}",
        fullscreen_args: "{rom}",
        tags: &["Emulator"],
    },
];

fn find_emulator_def(preset_id: &str) -> Option<&'static EmulatorDef> {
    EMULATOR_DEFS
        .iter()
        .find(|def| def.id.eq_ignore_ascii_case(preset_id.trim()))
}

pub fn list_emulator_presets() -> Vec<EmulatorPresetInfo> {
    let search_roots = collect_emulator_search_roots();
    let indexed_exes = index_emulator_binaries(&search_roots);

    EMULATOR_DEFS
        .iter()
        .map(|def| {
            let detected_exe_path = def
                .exe_names
                .iter()
                .find_map(|exe| indexed_exes.iter().find(|p| file_name_matches(p, exe)))
                .map(|p| p.to_string_lossy().into_owned());

            let detected_rom_dirs = if def.id == "cemu" {
                detect_cemu_game_paths(detected_exe_path.as_deref().map(Path::new))
                    .into_iter()
                    .filter(|p| p.is_dir())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect()
            } else {
                Vec::new()
            };

            EmulatorPresetInfo {
                id: def.id.to_string(),
                name: def.name.to_string(),
                system: def.system.to_string(),
                extensions: def.extensions.iter().map(|s| (*s).to_string()).collect(),
                default_args_template: def.default_args.to_string(),
                fullscreen_args_template: def.fullscreen_args.to_string(),
                default_tags: def.tags.iter().map(|s| (*s).to_string()).collect(),
                detected_exe_path,
                detected_rom_dirs,
            }
        })
        .collect()
}

fn collect_emulator_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for var in ["LOCALAPPDATA", "APPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Ok(val) = std::env::var(var) {
                if !val.trim().is_empty() {
                    roots.push(PathBuf::from(val));
                }
            }
        }
        for drive in ["C", "D", "E"] {
            for sub in ["Emulators", "Emulatoren", "Games", "Spiele", "Tools"] {
                roots.push(PathBuf::from(format!(r"{drive}:\{sub}")));
            }
        }
        if let Some(doc) = dirs::document_dir() {
            roots.push(doc);
        }
        if let Some(desk) = dirs::desktop_dir() {
            roots.push(desk);
        }
        if let Some(dl) = dirs::download_dir() {
            roots.push(dl);
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Desktop"));
            roots.push(home.join("Documents"));
            roots.push(home.join("Dokumente"));
            roots.push(home.join("Downloads"));
            roots.push(home.join("Emulators"));
            roots.push(home.join("Games"));
            roots.push(home.join("OneDrive").join("Dokumente"));
            roots.push(home.join("OneDrive").join("Documents"));
            roots.push(home.join("OneDrive").join("Desktop"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        roots.push(PathBuf::from("/usr/bin"));
        roots.push(PathBuf::from("/usr/local/bin"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".local/bin"));
            roots.push(home.join("Applications"));
            roots.push(home.join("Emulators"));
            roots.push(home.join("Games"));
        }
    }

    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|p| p.is_dir())
        .filter(|p| seen.insert(normalize_key(p)))
        .collect()
}

fn index_emulator_binaries(roots: &[PathBuf]) -> Vec<PathBuf> {
    let wanted: HashSet<String> = EMULATOR_DEFS
        .iter()
        .flat_map(|d| d.exe_names.iter())
        .map(|name| name.to_ascii_lowercase())
        .collect();

    let mut found = Vec::new();
    for root in roots {
        scan_dir_for_names(root, 0, 3, &wanted, &mut found);
    }
    found
}

fn scan_dir_for_names(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    wanted_lower: &HashSet<String>,
    out: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if wanted_lower.contains(&name.to_ascii_lowercase()) {
                    out.push(path);
                }
            }
        } else if ft.is_dir() && depth < max_depth {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = dir_name.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "node_modules"
                        | ".git"
                        | "target"
                        | "content"
                        | "shadercache"
                        | "graphicpacks"
                        | "mlc01"
                        | "windows"
                        | "$recycle.bin"
                ) {
                    continue;
                }
            }
            scan_dir_for_names(&path, depth + 1, max_depth, wanted_lower, out);
        }
    }
}

fn file_name_matches(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case(expected))
}

pub fn detect_cemu_game_paths(cemu_exe: Option<&Path>) -> Vec<PathBuf> {
    let mut xml_candidates = Vec::new();
    let mut raw_paths = Vec::new();

    if let Some(exe) = cemu_exe {
        if let Some(dir) = exe.parent() {
            xml_candidates.push(dir.join("settings.xml"));
            xml_candidates.push(dir.join("portable").join("settings.xml"));
            raw_paths.push(dir.join("games"));
            raw_paths.push(dir.join("roms"));
            raw_paths.push(dir.join("mlc01").join("usr").join("title").join("00050000"));
        }
    }
    if let Some(config) = dirs::config_dir() {
        xml_candidates.push(config.join("Cemu").join("settings.xml"));
    }
    if let Some(data_local) = dirs::data_local_dir() {
        xml_candidates.push(data_local.join("Cemu").join("settings.xml"));
    }

    for xml_path in xml_candidates {
        if !xml_path.is_file() {
            continue;
        }
        if let Ok(xml) = fs::read_to_string(&xml_path) {
            for p in parse_cemu_settings_xml(&xml) {
                raw_paths.push(p);
            }
        }
    }

    if let Some(config) = dirs::config_dir() {
        raw_paths.push(
            config
                .join("Cemu")
                .join("mlc01")
                .join("usr")
                .join("title")
                .join("00050000"),
        );
    }
    if let Some(data_local) = dirs::data_local_dir() {
        raw_paths.push(
            data_local
                .join("Cemu")
                .join("mlc01")
                .join("usr")
                .join("title")
                .join("00050000"),
        );
    }

    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for p in raw_paths {
        if !p.is_dir() {
            continue;
        }
        let key = normalize_key(&p);
        if seen.insert(key) {
            paths.push(p);
        }
    }
    paths
}

pub fn parse_cemu_settings_xml(xml: &str) -> Vec<PathBuf> {
    let Some(block) = extract_xml_tag_value(xml, "GamePaths") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut rest = block.as_str();
    while let Some(start) = find_ci(rest, "<Entry>") {
        let after_start = &rest[start + "<Entry>".len()..];
        let Some(end) = find_ci(after_start, "</Entry>") else {
            break;
        };
        let raw = decode_xml_entities(&after_start[..end]);
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            out.push(PathBuf::from(trimmed));
        }
        rest = &after_start[end + "</Entry>".len()..];
    }
    out
}

pub fn parse_cemu_meta_xml(xml: &str) -> Option<String> {
    for tag in ["longname_de", "longname_en", "shortname_de", "shortname_en"] {
        if let Some(raw) = extract_xml_tag_value(xml, tag) {
            let decoded = decode_xml_entities(&raw);
            let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
            if !collapsed.is_empty() {
                return Some(collapsed);
            }
        }
    }
    None
}

fn extract_xml_tag_value(xml: &str, tag: &str) -> Option<String> {
    let cleaned = strip_xml_comments(xml);
    let open_prefix = format!("<{tag}");
    let open_prefix_lower = open_prefix.to_ascii_lowercase();
    let cleaned_lower = cleaned.to_ascii_lowercase();

    let mut search_from = 0usize;
    while let Some(rel) = cleaned_lower[search_from..].find(&open_prefix_lower) {
        let open_idx = search_from + rel;
        let after_name_idx = open_idx + open_prefix.len();
        let next_ch = cleaned[after_name_idx..].chars().next();
        if !next_ch.is_some_and(|c| c == '>' || c == '/' || c.is_ascii_whitespace()) {
            search_from = after_name_idx;
            continue;
        }
        let after_open = &cleaned[after_name_idx..];
        let gt_rel = after_open.find('>')?;
        let before_gt = after_open[..gt_rel].trim_end();
        if before_gt.ends_with('/') {
            return Some(String::new());
        }
        let content_start = &after_open[gt_rel + 1..];
        let close_tag = format!("</{tag}>");
        let close_idx = find_ci(content_start, &close_tag)?;
        return Some(content_start[..close_idx].to_string());
    }
    None
}

fn strip_xml_comments(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        if let Some(end) = after.find("-->") {
            rest = &after[end + 3..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub fn build_emulator_launch_args(template: &str, rom_path: &str) -> String {
    let escaped = rom_path.replace('"', "\\\"");
    let quoted = if escaped.ends_with('\\') {
        format!("\"{escaped}\\\"")
    } else {
        format!("\"{escaped}\"")
    };
    if template.contains("{rom}") {
        template.replace("{rom}", &quoted)
    } else if template.trim().is_empty() {
        quoted
    } else {
        format!("{} {quoted}", template.trim())
    }
}

pub fn clean_game_title(raw: &str) -> String {
    let mut text = raw.trim().to_string();

    if let Some((stem, ext)) = text.rsplit_once('.') {
        let ext_lower = ext.to_ascii_lowercase();
        let known_rom = EMULATOR_DEFS
            .iter()
            .flat_map(|d| d.extensions.iter())
            .any(|e| *e == ext_lower);
        if known_rom || matches!(ext_lower.as_str(), "exe" | "desktop" | "appimage" | "lnk") {
            text = stem.to_string();
        }
    }

    text = strip_bracket_groups(&text, '[', ']');

    text = strip_metadata_parens(&text);

    for suffix in [
        "-Win64-Shipping",
        "-WinGDK-Shipping",
        "-Win32-Shipping",
        "_Win64",
        "_Win32",
        "_x64",
        "_x86",
    ] {
        if let Some(pos) = find_ci(&text, suffix) {
            if pos + suffix.len() == text.len() {
                text.truncate(pos);
            }
        }
    }

    let replaced = text.replace('_', " ");
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim_matches(|c| c == '-' || c == ' ').to_string()
}

fn strip_bracket_groups(input: &str, open: char, close: char) -> String {
    let mut out = String::with_capacity(input.len());
    let mut depth = 0usize;
    for ch in input.chars() {
        if ch == open {
            depth += 1;
        } else if ch == close && depth > 0 {
            depth -= 1;
        } else if depth == 0 {
            out.push(ch);
        }
    }
    out
}

fn strip_metadata_parens(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '(' {
            if let Some(rel_end) = chars[i + 1..].iter().position(|&c| c == ')') {
                let inner: String = chars[i + 1..i + 1 + rel_end].iter().collect();
                if is_rom_metadata_tag(&inner) {
                    i += rel_end + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn is_rom_metadata_tag(inner: &str) -> bool {
    let lower = inner.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return true;
    }
    let exact = [
        "usa", "europe", "eur", "eu", "us", "jp", "jpn", "japan", "world", "germany", "de",
        "france", "fr", "spain", "italy", "uk", "korea", "asia", "pal", "ntsc", "ntsc-u",
        "ntsc-j", "proto", "beta", "demo", "sample", "kiosk", "unk", "update", "dlc",
    ];
    if exact.contains(&lower.as_str()) {
        return true;
    }
    lower.starts_with("rev ")
        || lower.starts_with("v0")
        || lower.starts_with("v1")
        || lower.starts_with("v2")
        || lower.starts_with("en,")
        || lower.starts_with("en+")
        || lower.starts_with("disc ")
        || lower.starts_with("cd ")
        || lower.contains("europe")
        || lower.contains("usa")
        || lower.contains("wiiu")
}

pub fn find_nearby_cover(dir: &Path) -> Option<PathBuf> {
    let mut dirs_to_check = vec![dir.to_path_buf()];
    if let Some(parent) = dir.parent() {
        dirs_to_check.push(parent.to_path_buf());
        if let Some(grandparent) = parent.parent() {
            dirs_to_check.push(grandparent.to_path_buf());
        }
    }

    for candidate_dir in dirs_to_check {
        for base in COVER_BASE_NAMES {
            for ext in COVER_EXTS {
                let file = candidate_dir.join(format!("{base}.{ext}"));
                if file.is_file() {
                    return Some(file);
                }
            }
        }
    }
    None
}

pub fn scan_emulator_folder(
    preset_id: &str,
    emulator_path: Option<&str>,
    rom_dir: &Path,
    fullscreen: bool,
    existing_keys: &ExistingGameKeys,
) -> AppResult<Vec<DiscoveredGame>> {
    if !rom_dir.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "ROM directory does not exist: {}",
            rom_dir.display()
        )));
    }

    let def = find_emulator_def(preset_id)
        .ok_or_else(|| AppError::InvalidInput(format!("Unknown emulator preset: {preset_id}")))?;

    let resolved_exe = emulator_path
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            list_emulator_presets()
                .into_iter()
                .find(|p| p.id == def.id)
                .and_then(|p| p.detected_exe_path)
        });

    let working_dir = resolved_exe
        .as_deref()
        .and_then(|exe| Path::new(exe).parent())
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned());

    let args_template = if fullscreen {
        def.fullscreen_args
    } else {
        def.default_args
    };

    let mut files = Vec::new();
    collect_files_recursive(rom_dir, 0, 7, 5000, &mut files);

    let cue_dirs: HashSet<String> = files
        .iter()
        .filter(|p| has_ext(p, "cue"))
        .filter_map(|p| p.parent())
        .map(normalize_key)
        .collect();

    let mut results = Vec::new();
    let mut seen_roms = HashSet::new();
    let mut seen_titles = HashSet::new();

    for path in files {
        let Some(ext) = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) else {
            continue;
        };

        if def.id == "custom" {
            if NON_ROM_EXTS.contains(&ext.as_str()) {
                continue;
            }
        } else if !def.extensions.contains(&ext.as_str()) {
            continue;
        }

        if def.id == "rpcs3" && ext == "bin" && !is_rpcs3_eboot(&path) {
            continue;
        }

        if (def.id == "duckstation" || def.id == "pcsx2")
            && ext == "bin"
            && path.parent().is_some_and(|dir| cue_dirs.contains(&normalize_key(dir)))
        {
            continue;
        }

        if def.id == "cemu" && ext == "rpx" && is_cemu_update_or_dlc_path(&path) {
            continue;
        }

        let rom_key = normalize_key(&path);
        if !seen_roms.insert(rom_key) {
            continue;
        }

        let base_title = derive_emulator_game_title(def.id, &path);
        let disc_label = extract_disc_label(&path);
        let title = if let Some(ref disc) = disc_label {
            if base_title.to_ascii_lowercase().contains(&disc.to_ascii_lowercase()) {
                base_title
            } else {
                format!("{base_title} ({disc})")
            }
        } else {
            base_title
        };
        let dedup_key = if def.id == "cemu" {
            title.to_lowercase()
        } else {
            format!("{}:{}", title.to_lowercase(), disc_label.unwrap_or_default().to_lowercase())
        };
        if !seen_titles.insert(dedup_key) {
            continue;
        }

        let rom_str = path.to_string_lossy().into_owned();
        let launch_args = build_emulator_launch_args(args_template, &rom_str);
        let cover_dir = if def.id == "cemu" && ext == "rpx" {
            path.parent().and_then(Path::parent).unwrap_or(rom_dir)
        } else {
            path.parent().unwrap_or(rom_dir)
        };

        let cover_path = find_nearby_cover(cover_dir)
            .or_else(|| {
                if def.id == "cemu" {
                    find_cemu_cover(&path, &title, resolved_exe.as_deref())
                } else {
                    None
                }
            })
            .map(|p| p.to_string_lossy().into_owned());

        let already_imported =
            existing_keys.contains_exe_args(resolved_exe.as_deref(), Some(&launch_args));

        results.push(DiscoveredGame {
            title,
            platform: Platform::Emulator,
            source: def.name.to_string(),
            steam_app_id: None,
            executable_path: resolved_exe.clone(),
            install_dir: path.parent().map(|p| p.to_string_lossy().into_owned()),
            launch_args: Some(launch_args),
            working_dir: working_dir.clone(),
            cover_path,
            cover_url: None,
            hero_url: None,
            tags: def.tags.iter().map(|s| (*s).to_string()).collect(),
            already_imported,
        });
    }

    results.sort_by_key(|g| g.title.to_lowercase());
    Ok(results)
}

fn extract_disc_label(path: &Path) -> Option<String> {
    let name = path.file_stem().and_then(|s| s.to_str())?;
    let lower = name.to_ascii_lowercase();
    for prefix in ["(disc ", "(cd "] {
        if let Some(start) = lower.find(prefix) {
            if let Some(rel_end) = name[start + 1..].find(')') {
                let inner = name[start + 1..start + 1 + rel_end].trim();
                if !inner.is_empty() {
                    return Some(inner.to_string());
                }
            }
        }
    }
    None
}

fn collect_files_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_files: usize,
    out: &mut Vec<PathBuf>,
) {
    if out.len() >= max_files {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if out.len() >= max_files {
            return;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_file() {
            out.push(path);
        } else if ft.is_dir() && depth < max_depth {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = dir_name.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "content"
                        | "shadercache"
                        | "graphicpacks"
                        | "controllerprofiles"
                        | "memorysearcher"
                        | "resources"
                        | "gameprofiles"
                        | "dlcs"
                        | "updates"
                        | "updates and dlc"
                        | "0005000c"
                        | "0005000e"
                        | "node_modules"
                        | ".git"
                        | "target"
                ) {
                    continue;
                }
            }
            collect_files_recursive(&path, depth + 1, max_depth, max_files, out);
        }
    }
}

fn has_ext(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(expected))
}

fn is_rpcs3_eboot(path: &Path) -> bool {
    let is_eboot = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("EBOOT.BIN"));
    if !is_eboot {
        return false;
    }
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("USRDIR"))
}

fn is_cemu_update_or_dlc_path(rpx_path: &Path) -> bool {
    let lower = rpx_path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    if lower.contains("/0005000e/")
        || lower.contains("/0005000c/")
        || lower.contains("/updates and dlc/")
        || lower.contains("/updates/")
        || lower.contains("/dlcs/")
        || lower.contains("(dlc)")
        || lower.contains("(update)")
    {
        return true;
    }
    if let Some(game_root) = rpx_path.parent().and_then(Path::parent) {
        let meta_path = game_root.join("meta").join("meta.xml");
        if meta_path.is_file() {
            if let Ok(xml) = fs::read_to_string(&meta_path) {
                if let Some(tid) = extract_xml_tag_value(&xml, "title_id") {
                    let tid_lower = tid.trim().to_ascii_lowercase();
                    if tid_lower.starts_with("0005000e") || tid_lower.starts_with("0005000c") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn derive_emulator_game_title(preset_id: &str, rom_path: &Path) -> String {
    if preset_id == "cemu" && has_ext(rom_path, "rpx") {
        if let Some(game_root) = rom_path.parent().and_then(Path::parent) {
            let meta_path = game_root.join("meta").join("meta.xml");
            if meta_path.is_file() {
                if let Ok(xml) = fs::read_to_string(&meta_path) {
                    if let Some(meta_title) = parse_cemu_meta_xml(&xml) {
                        return meta_title;
                    }
                }
            }
            if let Some(folder_name) = game_root.file_name().and_then(|n| n.to_str()) {
                let cleaned = clean_game_title(folder_name);
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
    }

    if preset_id == "rpcs3" && is_rpcs3_eboot(rom_path) {
        if let Some(game_root) = rom_path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
        {
            if let Some(folder_name) = game_root.file_name().and_then(|n| n.to_str()) {
                let cleaned = clean_game_title(folder_name);
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
    }

    let fallback = rom_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Game");
    let cleaned = clean_game_title(fallback);
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

pub fn scan_pc_games_folder(
    folder: &Path,
    existing_keys: &ExistingGameKeys,
) -> AppResult<Vec<DiscoveredGame>> {
    if !folder.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "Directory does not exist: {}",
            folder.display()
        )));
    }

    let entries = fs::read_dir(folder)?;
    let mut results = Vec::new();

    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();

        if ft.is_file() && has_ext(&path, "desktop") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(preview) = parse_desktop_entry(&content) {
                    let already = existing_keys.contains_exe_args(
                        Some(&preview.executable_path),
                        preview.launch_args.as_deref(),
                    );
                    results.push(DiscoveredGame {
                        title: preview.title,
                        platform: Platform::Manual,
                        source: "Folder Scan".to_string(),
                        steam_app_id: None,
                        executable_path: Some(preview.executable_path),
                        install_dir: None,
                        launch_args: preview.launch_args,
                        working_dir: preview.working_dir,
                        cover_path: None,
                        cover_url: None,
                        hero_url: None,
                        tags: Vec::new(),
                        already_imported: already,
                    });
                }
            }
            continue;
        }

        if !ft.is_dir() {
            continue;
        }

        let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if folder_name.starts_with('.') {
            continue;
        }

        if let Some(exe_path) = pick_best_pc_executable(&path, folder_name) {
            let exe_str = exe_path.to_string_lossy().into_owned();
            let title = clean_game_title(folder_name);
            let working_dir = exe_path.parent().map(|p| p.to_string_lossy().into_owned());
            let already = existing_keys.contains_exe_args(Some(&exe_str), None);

            results.push(DiscoveredGame {
                title: if title.is_empty() {
                    folder_name.to_string()
                } else {
                    title
                },
                platform: Platform::Manual,
                source: "Folder Scan".to_string(),
                steam_app_id: None,
                executable_path: Some(exe_str),
                install_dir: Some(path.to_string_lossy().into_owned()),
                launch_args: None,
                working_dir,
                cover_path: find_nearby_cover(&path).map(|p| p.to_string_lossy().into_owned()),
                cover_url: None,
                hero_url: None,
                tags: Vec::new(),
                already_imported: already,
            });
        }
    }

    results.sort_by_key(|g| g.title.to_lowercase());
    Ok(results)
}

fn pick_best_pc_executable(game_dir: &Path, folder_name: &str) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_pc_executables_recursive(game_dir, 0, 4, 500, &mut files);

    let folder_norm = normalize_alnum(folder_name);
    let mut best: Option<(i64, PathBuf)> = None;

    for file in files {
        let Some(name) = file.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_candidate_pc_binary(&file, name) || is_ignored_pc_binary(name) {
            continue;
        }

        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let stem_norm = normalize_alnum(stem);
        let lower_name = name.to_ascii_lowercase();
        let lower_path = file.to_string_lossy().replace('\\', "/").to_ascii_lowercase();

        let mut score: i64 = 10;
        if !folder_norm.is_empty()
            && !stem_norm.is_empty()
            && (stem_norm == folder_norm
                || stem_norm.contains(&folder_norm)
                || folder_norm.contains(&stem_norm))
        {
            score += 55;
        }
        if lower_name.ends_with("-win64-shipping.exe")
            || lower_name.ends_with("-wingdk-shipping.exe")
        {
            score += 50;
        }
        if file.parent() == Some(game_dir) || lower_path.contains("/binaries/win64/") {
            score += 20;
        }
        if let Ok(meta) = fs::metadata(&file) {
            let mb = (meta.len() / (1024 * 1024)) as i64;
            score += mb.min(35);
        }
        for penalty in [
            "launcher",
            "config",
            "settings",
            "editor",
            "server",
            "dedicated",
            "benchmark",
            "updater",
            "helper",
            "tool",
        ] {
            if lower_name.contains(penalty) {
                score -= 35;
            }
        }

        if best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
            best = Some((score, file));
        }
    }

    best.map(|(_, path)| path)
}

fn collect_pc_executables_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_binaries: usize,
    out: &mut Vec<PathBuf>,
) {
    if out.len() >= max_binaries {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= max_binaries {
            return;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if is_candidate_pc_binary(&path, name) && !is_ignored_pc_binary(name) {
                    out.push(path);
                }
            }
        } else if ft.is_dir() && depth < max_depth {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = dir_name.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "node_modules" | ".git" | "target" | "shadercache" | "__pycache__"
                ) {
                    continue;
                }
            }
            collect_pc_executables_recursive(&path, depth + 1, max_depth, max_binaries, out);
        }
    }
}

fn is_candidate_pc_binary(path: &Path, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".exe") || lower.ends_with(".appimage") || lower.ends_with(".sh") {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.extension().is_none() {
            if let Ok(meta) = fs::metadata(path) {
                return meta.permissions().mode() & 0o111 != 0;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    false
}

fn is_ignored_pc_binary(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    JUNK_EXE_PREFIXES.iter().any(|p| lower.starts_with(p))
        || JUNK_EXE_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

fn normalize_alnum(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub fn parse_epic_manifest(json_str: &str) -> Option<DiscoveredGame> {
    let val: Value = serde_json::from_str(json_str).ok()?;
    if val
        .get("bIsIncompleteInstall")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    let app_name = val.get("AppName").and_then(Value::as_str).unwrap_or("");
    let catalog_ns = val
        .get("CatalogNamespace")
        .and_then(Value::as_str)
        .unwrap_or("");
    if app_name.starts_with("UE_") || catalog_ns.eq_ignore_ascii_case("ue") {
        return None;
    }

    let title = val
        .get("DisplayName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();

    let install_location = val
        .get("InstallLocation")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let launch_exe = val
        .get("LaunchExecutable")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let full_exe = Path::new(install_location).join(launch_exe);
    let working_dir = full_exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned());

    Some(DiscoveredGame {
        title,
        platform: Platform::Manual,
        source: "Epic Games".to_string(),
        steam_app_id: None,
        executable_path: Some(full_exe.to_string_lossy().into_owned()),
        install_dir: Some(install_location.to_string()),
        launch_args: val
            .get("LaunchCommand")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        working_dir,
        cover_path: None,
        cover_url: None,
        hero_url: None,
        tags: vec!["Epic Games".to_string()],
        already_imported: false,
    })
}

pub fn scan_pc_stores(
    custom_steam_path: Option<&Path>,
    existing_keys: &ExistingGameKeys,
) -> Vec<DiscoveredGame> {
    let mut results = Vec::new();

    let steam_scan = steam::scan_steam_libraries(custom_steam_path);
    for app in steam_scan.apps {
        let already = existing_keys.contains_steam(&app.app_id);
        results.push(DiscoveredGame {
            title: app.name,
            platform: Platform::Steam,
            source: "Steam".to_string(),
            steam_app_id: Some(app.app_id),
            executable_path: None,
            install_dir: Some(app.install_dir.to_string_lossy().into_owned()),
            launch_args: None,
            working_dir: None,
            cover_path: None,
            cover_url: Some(app.cover_url),
            hero_url: Some(app.hero_url),
            tags: Vec::new(),
            already_imported: already,
        });
    }

    #[cfg(target_os = "windows")]
    {
        for mut item in scan_windows_epic_manifests() {
            if let Some(ref exe) = item.executable_path {
                if Path::new(exe).is_file() {
                    item.already_imported = existing_keys
                        .contains_exe_args(Some(exe), item.launch_args.as_deref());
                    item.cover_path = item
                        .install_dir
                        .as_deref()
                        .and_then(|dir| find_nearby_cover(Path::new(dir)))
                        .map(|p| p.to_string_lossy().into_owned());
                    results.push(item);
                }
            }
        }

        for mut item in scan_windows_gog_registry() {
            if let Some(ref exe) = item.executable_path {
                if Path::new(exe).is_file() {
                    item.already_imported = existing_keys
                        .contains_exe_args(Some(exe), item.launch_args.as_deref());
                    item.cover_path = item
                        .install_dir
                        .as_deref()
                        .and_then(|dir| find_nearby_cover(Path::new(dir)))
                        .map(|p| p.to_string_lossy().into_owned());
                    results.push(item);
                }
            }
        }

        for mut item in scan_windows_minecraft() {
            item.already_imported = existing_keys.contains_exe_args(
                item.executable_path.as_deref(),
                item.launch_args.as_deref(),
            );
            results.push(item);
        }
    }

    let mut seen_emu_titles = HashSet::new();
    for preset in list_emulator_presets() {
        let Some(ref exe_path) = preset.detected_exe_path else {
            continue;
        };
        for rom_dir in &preset.detected_rom_dirs {
            if let Ok(found) = scan_emulator_folder(
                &preset.id,
                Some(exe_path),
                Path::new(rom_dir),
                true,
                existing_keys,
            ) {
                for item in found {
                    let key = format!("{}:{}", preset.id, item.title.to_lowercase());
                    if seen_emu_titles.insert(key) {
                        results.push(item);
                    }
                }
            }
        }
    }

    results.sort_by_key(|g| g.title.to_lowercase());
    results
}

#[cfg(target_os = "windows")]
fn scan_windows_minecraft() -> Vec<DiscoveredGame> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let mut out = Vec::new();
    let mut found_launcher = false;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(packages) = hkcu.open_subkey(
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages",
    ) {
        let mut found_uwp = false;
        for pkg_name in packages.enum_keys().flatten() {
            let Ok(pkg) = packages.open_subkey(&pkg_name) else {
                continue;
            };
            let root: Option<String> = pkg
                .get_value::<String, _>("PackageRootFolder")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            if pkg_name.starts_with("Microsoft.4297127D64EC6_") && !found_launcher {
                found_launcher = true;
                let cover_path = root.as_deref().and_then(|r| {
                    find_uwp_logo(Path::new(r), &["GraphicsLogo.png", "Logo.png", "StoreLogo.png"])
                });
                let has_java_profiles = dirs::config_dir()
                    .map(|c| c.join(".minecraft").join("launcher_profiles.json").is_file())
                    .unwrap_or(false);
                let mut tags = vec!["Minecraft".to_string()];
                if has_java_profiles {
                    tags.push("Java Edition".to_string());
                }

                out.push(DiscoveredGame {
                    title: "Minecraft Launcher".to_string(),
                    platform: Platform::Manual,
                    source: "Minecraft".to_string(),
                    steam_app_id: None,
                    executable_path: Some("explorer.exe".to_string()),
                    install_dir: root,
                    launch_args: Some(
                        r"shell:AppsFolder\Microsoft.4297127D64EC6_8wekyb3d8bbwe!Minecraft"
                            .to_string(),
                    ),
                    working_dir: None,
                    cover_path,
                    cover_url: None,
                    hero_url: None,
                    tags,
                    already_imported: false,
                });
            } else if pkg_name.starts_with("Microsoft.MinecraftUWP_") && !found_uwp {
                found_uwp = true;
                let cover_path = root.as_deref().and_then(|r| {
                    find_uwp_logo(Path::new(r), &["LargeLogo.png", "Logo.png", "StoreLogo.png"])
                });
                out.push(DiscoveredGame {
                    title: "Minecraft for Windows".to_string(),
                    platform: Platform::Manual,
                    source: "Minecraft".to_string(),
                    steam_app_id: None,
                    executable_path: Some("explorer.exe".to_string()),
                    install_dir: root,
                    launch_args: Some(
                        r"shell:AppsFolder\Microsoft.MinecraftUWP_8wekyb3d8bbwe!Game".to_string(),
                    ),
                    working_dir: None,
                    cover_path,
                    cover_url: None,
                    hero_url: None,
                    tags: vec!["Minecraft".to_string(), "Bedrock".to_string()],
                    already_imported: false,
                });
            }
        }
    }

    if !found_launcher {
        for candidate in [
            r"C:\Program Files (x86)\Minecraft Launcher\MinecraftLauncher.exe",
            r"C:\XboxGames\Minecraft Launcher\Content\Minecraft.exe",
        ] {
            let p = Path::new(candidate);
            if p.is_file() {
                let dir = p.parent().map(|d| d.to_string_lossy().into_owned());
                let cover = p.parent().and_then(find_nearby_cover).map(|c| c.to_string_lossy().into_owned());
                out.push(DiscoveredGame {
                    title: "Minecraft Launcher".to_string(),
                    platform: Platform::Manual,
                    source: "Minecraft".to_string(),
                    steam_app_id: None,
                    executable_path: Some(candidate.to_string()),
                    install_dir: dir.clone(),
                    launch_args: None,
                    working_dir: dir,
                    cover_path: cover,
                    cover_url: None,
                    hero_url: None,
                    tags: vec!["Minecraft".to_string()],
                    already_imported: false,
                });
                break;
            }
        }
    }

    out
}

#[cfg(target_os = "windows")]
fn find_uwp_logo(root: &Path, names: &[&str]) -> Option<String> {
    for name in names {
        let p = root.join(name);
        if p.is_file() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn scan_windows_epic_manifests() -> Vec<DiscoveredGame> {
    let manifests_dir = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(entries) = fs::read_dir(manifests_dir) else {
        return Vec::new();
    };
    let catalog_images = load_epic_catalog_images();
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !has_ext(&path, "item") || !path.is_file() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Some(mut item) = parse_epic_manifest(&content) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    let item_id = val
                        .get("CatalogItemId")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or("");
                    if let Some((cover_url, hero_url)) = catalog_images
                        .get(item_id)
                        .or_else(|| catalog_images.get(&item.title.to_lowercase()))
                    {
                        if item.cover_url.is_none() {
                            item.cover_url.clone_from(cover_url);
                        }
                        if item.hero_url.is_none() {
                            item.hero_url.clone_from(hero_url);
                        }
                    }
                }
                out.push(item);
            }
        }
    }
    out
}

#[cfg(target_os = "windows")]
fn load_epic_catalog_images() -> HashMap<String, (Option<String>, Option<String>)> {
    let mut map = HashMap::new();
    let cache_path = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Catalog\catcache.bin");
    let Ok(raw) = fs::read_to_string(cache_path) else {
        return map;
    };
    let trimmed = raw.trim();
    let json_bytes = if trimmed.starts_with('[') {
        trimmed.as_bytes().to_vec()
    } else if let Some(decoded) = decode_base64(trimmed) {
        decoded
    } else {
        return map;
    };

    let Ok(Value::Array(items)) = serde_json::from_slice::<Value>(&json_bytes) else {
        return map;
    };

    for entry in items {
        let Some(images) = entry.get("keyImages").and_then(Value::as_array) else {
            continue;
        };
        let mut tall: Option<String> = None;
        let mut offer_tall: Option<String> = None;
        let mut thumb: Option<String> = None;
        let mut wide: Option<String> = None;
        let mut featured: Option<String> = None;

        for img in images {
            let kind = img.get("type").and_then(Value::as_str).unwrap_or("");
            let Some(url) = img
                .get("url")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            match kind {
                "DieselGameBoxTall" => tall = Some(url.to_string()),
                "OfferImageTall" => offer_tall = Some(url.to_string()),
                "Thumbnail" => thumb = Some(url.to_string()),
                "DieselGameBox" => wide = Some(url.to_string()),
                "OfferImageWide" => {
                    if wide.is_none() {
                        wide = Some(url.to_string());
                    }
                }
                "Featured" => featured = Some(url.to_string()),
                _ => {}
            }
        }

        let cover = tall.or(offer_tall).or(thumb).or_else(|| wide.clone());
        let hero = wide.or(featured);
        if cover.is_none() && hero.is_none() {
            continue;
        }

        if let Some(id) = entry.get("id").and_then(Value::as_str).map(str::trim) {
            if !id.is_empty() {
                map.insert(id.to_string(), (cover.clone(), hero.clone()));
            }
        }
        if let Some(title) = entry.get("title").and_then(Value::as_str).map(str::trim) {
            if !title.is_empty() {
                map.entry(title.to_lowercase()).or_insert((cover, hero));
            }
        }
    }

    map
}

#[cfg(target_os = "windows")]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u8 = 0;
    for b in input.bytes() {
        if b.is_ascii_whitespace() || b == b'=' {
            continue;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        buf = (buf << 6) | u32::from(val);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

fn find_cemu_cover(rom_path: &Path, title: &str, cemu_exe: Option<&str>) -> Option<PathBuf> {
    let mut meta_dirs = Vec::new();
    let mut title_id_low: Option<String> = None;

    if has_ext(rom_path, "rpx") {
        if let Some(game_root) = rom_path.parent().and_then(Path::parent) {
            let local_meta = game_root.join("meta");
            if local_meta.is_dir() {
                let xml_path = local_meta.join("meta.xml");
                if let Ok(xml) = fs::read_to_string(&xml_path) {
                    if let Some(tid) = extract_xml_tag_value(&xml, "title_id") {
                        let clean_tid = tid.trim().to_ascii_lowercase();
                        let chars: Vec<char> = clean_tid.chars().collect();
                        if chars.len() >= 8 {
                            let tail: String = chars[chars.len() - 8..].iter().collect();
                            title_id_low = Some(tail);
                        }
                    }
                }
                meta_dirs.push(local_meta);
            }
        }
    }

    let mut mlc_bases = Vec::new();
    if let Some(exe) = cemu_exe {
        if let Some(dir) = Path::new(exe).parent() {
            mlc_bases.push(dir.join("mlc01").join("usr").join("title").join("00050000"));
        }
    }
    if let Some(config) = dirs::config_dir() {
        mlc_bases.push(
            config
                .join("Cemu")
                .join("mlc01")
                .join("usr")
                .join("title")
                .join("00050000"),
        );
    }
    if let Some(data_local) = dirs::data_local_dir() {
        mlc_bases.push(
            data_local
                .join("Cemu")
                .join("mlc01")
                .join("usr")
                .join("title")
                .join("00050000"),
        );
    }

    let title_norm = normalize_alnum(title);
    for base in mlc_bases {
        if !base.is_dir() {
            continue;
        }
        if let Some(ref tid_low) = title_id_low {
            let candidate = base.join(tid_low).join("meta");
            if candidate.is_dir() {
                meta_dirs.push(candidate);
            }
        }
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.flatten() {
                let mdir = entry.path().join("meta");
                if !mdir.is_dir() {
                    continue;
                }
                let xml_path = mdir.join("meta.xml");
                if let Ok(xml) = fs::read_to_string(&xml_path) {
                    if let Some(meta_title) = parse_cemu_meta_xml(&xml) {
                        if normalize_alnum(&meta_title) == title_norm {
                            meta_dirs.push(mdir);
                        }
                    }
                }
            }
        }
    }

    for meta_dir in meta_dirs {
        for name in [
            "cover.png",
            "cover.jpg",
            "bootTvTex.tga",
            "bootDrcTex.tga",
            "iconTex.tga",
        ] {
            let candidate = meta_dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn backfill_missing_covers(db: &crate::db::Database, covers_dir: &Path) -> usize {
    let Ok(games) = db.list_games() else {
        return 0;
    };

    #[cfg(target_os = "windows")]
    let epic_images = load_epic_catalog_images();
    #[cfg(target_os = "windows")]
    let epic_items = scan_windows_epic_manifests();

    let mut updated = 0usize;
    for game in games {
        if game.cover_url.is_some() {
            continue;
        }

        let mut next_cover: Option<String> = None;
        let mut next_cover_local = game.cover_is_local;
        let mut next_hero = game.hero_url.clone();

        #[cfg(target_os = "windows")]
        if game.platform == Platform::Manual {
            let game_exe_norm = game.executable_path.as_deref().map(normalize_key_str);
            let game_dir_norm = game.install_dir.as_deref().map(normalize_key_str);
            let matched = epic_items.iter().find(|item| {
                game_exe_norm
                    .as_deref()
                    .zip(item.executable_path.as_deref().map(normalize_key_str).as_deref())
                    .is_some_and(|(a, b)| a == b)
                    || game_dir_norm
                        .as_deref()
                        .zip(item.install_dir.as_deref().map(normalize_key_str).as_deref())
                        .is_some_and(|(a, b)| a == b)
            });

            if let Some(item) = matched {
                next_cover.clone_from(&item.cover_url);
                if next_hero.is_none() {
                    next_hero.clone_from(&item.hero_url);
                }
            } else if let Some((c, h)) = epic_images.get(&game.title.to_lowercase()) {
                next_cover.clone_from(c);
                if next_hero.is_none() {
                    next_hero.clone_from(h);
                }
            }
        }

        if next_cover.is_none() && game.platform == Platform::Emulator {
            if let Some(ref args) = game.launch_args {
                if let Ok(tokens) = crate::launcher::parse_launch_args(args) {
                    if let Some(rom_token) = tokens.last() {
                        let rom_path = Path::new(rom_token);
                        if let Some(cemu_cov) =
                            find_cemu_cover(rom_path, &game.title, game.executable_path.as_deref())
                        {
                            if let Ok(managed) =
                                crate::covers::copy_cover_to_dir(&cemu_cov, covers_dir)
                            {
                                next_cover = Some(managed);
                                next_cover_local = true;
                            }
                        }
                    }
                }
            }
        }

        if next_cover.is_some() {
            let input = crate::models::UpdateGameInput {
                id: game.id.clone(),
                title: game.title.clone(),
                platform: game.platform,
                steam_app_id: game.steam_app_id.clone(),
                executable_path: game.executable_path.clone(),
                install_dir: game.install_dir.clone(),
                launch_args: game.launch_args.clone(),
                working_dir: game.working_dir.clone(),
                cover_url: next_cover,
                cover_is_local: next_cover_local,
                hero_url: next_hero,
                is_favorite: game.is_favorite,
                tags: game.tags.clone(),
            };
            if db.update_game(input).is_ok() {
                updated += 1;
            }
        }
    }

    updated
}

#[cfg(target_os = "windows")]
fn normalize_key_str(raw: &str) -> String {
    raw.trim()
        .trim_end_matches(['/', '\\'])
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn scan_windows_gog_registry() -> Vec<DiscoveredGame> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut out = Vec::new();
    let mut seen_exes = HashSet::new();

    for root_key in [r"SOFTWARE\WOW6432Node\GOG.com\Games", r"SOFTWARE\GOG.com\Games"] {
        let Ok(games_key) = hklm.open_subkey(root_key) else {
            continue;
        };
        for sub_name in games_key.enum_keys().flatten() {
            let Ok(sub) = games_key.open_subkey(&sub_name) else {
                continue;
            };
            let title: String = match sub.get_value::<String, _>("gameName") {
                Ok(v) if !v.trim().is_empty() => v,
                _ => continue,
            };
            let exe: String = match sub.get_value::<String, _>("exe") {
                Ok(v) if !v.trim().is_empty() => v,
                _ => continue,
            };
            let exe_key = exe.replace('/', "\\").to_ascii_lowercase();
            if !seen_exes.insert(exe_key) {
                continue;
            }
            let install_dir: Option<String> = sub
                .get_value::<String, _>("path")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let working_dir = Path::new(&exe)
                .parent()
                .map(|p| p.to_string_lossy().into_owned());

            out.push(DiscoveredGame {
                title: title.trim().to_string(),
                platform: Platform::Manual,
                source: "GOG".to_string(),
                steam_app_id: None,
                executable_path: Some(exe),
                install_dir,
                launch_args: None,
                working_dir,
                cover_path: None,
                cover_url: None,
                hero_url: None,
                tags: vec!["GOG".to_string()],
                already_imported: false,
            });
        }
    }

    out
}

pub fn inspect_game_file(path: &Path) -> AppResult<GameFileInspection> {
    if !path.is_file() {
        return Err(AppError::InvalidInput(format!(
            "Selected file does not exist: {}",
            path.display()
        )));
    }

    let path_str = path.to_string_lossy().into_owned();
    let parent_dir = path.parent();

    if has_ext(path, "desktop") {
        let content = fs::read_to_string(path)?;
        let preview = parse_desktop_entry(&content)?;
        return Ok(GameFileInspection {
            title: preview.title,
            platform: Platform::Manual,
            detected_emulator_id: None,
            is_emulator_binary: false,
            is_rom_file: false,
            executable_path: Some(preview.executable_path),
            rom_path: None,
            install_dir: None,
            launch_args: preview.launch_args,
            working_dir: preview.working_dir,
            cover_path: parent_dir
                .and_then(find_nearby_cover)
                .map(|p| p.to_string_lossy().into_owned()),
            tags: Vec::new(),
        });
    }

    if let Some(def) = EMULATOR_DEFS
        .iter()
        .find(|d| d.exe_names.iter().any(|name| file_name_matches(path, name)))
    {
        return Ok(GameFileInspection {
            title: String::new(),
            platform: Platform::Emulator,
            detected_emulator_id: Some(def.id.to_string()),
            is_emulator_binary: true,
            is_rom_file: false,
            executable_path: Some(path_str),
            rom_path: None,
            install_dir: None,
            launch_args: None,
            working_dir: parent_dir.map(|p| p.to_string_lossy().into_owned()),
            cover_path: None,
            tags: def.tags.iter().map(|s| (*s).to_string()).collect(),
        });
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let presets = list_emulator_presets();
    let matched_rom_def = if is_rpcs3_eboot(path) {
        find_emulator_def("rpcs3")
    } else if ext == "bin" {
        None
    } else {
        EMULATOR_DEFS
            .iter()
            .filter(|d| d.extensions.contains(&ext.as_str()))
            .find(|d| {
                presets
                    .iter()
                    .any(|p| p.id == d.id && p.detected_exe_path.is_some())
            })
            .or_else(|| {
                EMULATOR_DEFS
                    .iter()
                    .find(|d| d.extensions.contains(&ext.as_str()))
            })
    };

    if let Some(def) = matched_rom_def {
        let detected_exe = presets
            .into_iter()
            .find(|p| p.id == def.id)
            .and_then(|p| p.detected_exe_path);
        let working_dir = detected_exe
            .as_deref()
            .and_then(|exe| Path::new(exe).parent())
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy().into_owned());
        let cover_dir = if def.id == "cemu" && ext == "rpx" {
            path.parent().and_then(Path::parent).or(parent_dir)
        } else {
            parent_dir
        };

        let title = derive_emulator_game_title(def.id, path);
        let cover_path = cover_dir
            .and_then(find_nearby_cover)
            .or_else(|| {
                if def.id == "cemu" {
                    find_cemu_cover(path, &title, detected_exe.as_deref())
                } else {
                    None
                }
            })
            .map(|p| p.to_string_lossy().into_owned());

        return Ok(GameFileInspection {
            title,
            platform: Platform::Emulator,
            detected_emulator_id: Some(def.id.to_string()),
            is_emulator_binary: false,
            is_rom_file: true,
            executable_path: detected_exe,
            rom_path: Some(path_str.clone()),
            install_dir: parent_dir.map(|p| p.to_string_lossy().into_owned()),
            launch_args: Some(build_emulator_launch_args(def.fullscreen_args, &path_str)),
            working_dir,
            cover_path,
            tags: def.tags.iter().map(|s| (*s).to_string()).collect(),
        });
    }

    let install_root = infer_pc_install_root(path);
    let raw_name = install_root
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|_| is_unreal_subpath(path))
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown Game")
        });

    let mut title = clean_game_title(raw_name);
    if title.is_empty() {
        title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Game")
            .to_string();
    }

    let cover_path = install_root
        .as_deref()
        .or(parent_dir)
        .and_then(find_nearby_cover)
        .map(|p| p.to_string_lossy().into_owned());

    Ok(GameFileInspection {
        title,
        platform: Platform::Manual,
        detected_emulator_id: None,
        is_emulator_binary: false,
        is_rom_file: false,
        executable_path: Some(path_str),
        rom_path: None,
        install_dir: install_root.map(|p| p.to_string_lossy().into_owned()),
        launch_args: None,
        working_dir: parent_dir.map(|p| p.to_string_lossy().into_owned()),
        cover_path,
        tags: Vec::new(),
    })
}

fn is_unreal_subpath(path: &Path) -> bool {
    let lower = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    lower.contains("/binaries/win64/") || lower.contains("/binaries/wingdk/")
}

fn infer_pc_install_root(exe_path: &Path) -> Option<PathBuf> {
    if is_unreal_subpath(exe_path) {

        let mut cur = exe_path.parent();
        while let Some(dir) = cur {
            if dir
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("Binaries"))
            {
                return dir.parent().and_then(Path::parent).map(Path::to_path_buf);
            }
            cur = dir.parent();
        }
    }
    exe_path.parent().map(Path::to_path_buf)
}

fn normalize_key(path: &Path) -> String {
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
    fn test_clean_game_title() {
        assert_eq!(
            clean_game_title("The Legend of Zelda - Breath of the Wild (USA) (En,Fr,Es) [00050000101C9400].wua"),
            "The Legend of Zelda - Breath of the Wild"
        );
        assert_eq!(
            clean_game_title("Mario_Kart_8_(Europe)_(Rev 1).wux"),
            "Mario Kart 8"
        );
        assert_eq!(
            clean_game_title("MyGame-Win64-Shipping.exe"),
            "MyGame"
        );
        assert_eq!(
            clean_game_title("Super Mario 3D World (2013)"),
            "Super Mario 3D World (2013)"
        );
    }

    #[test]
    fn test_parse_cemu_meta_xml() {
        let xml = r#"
        <?xml version="1.0" encoding="utf-8"?>
        <menu type="complex" access="777">
          <longname_en type="string" length="512">The Legend of Zelda:
Breath of the Wild &amp; Expansion</longname_en>
          <longname_de type="string" length="512">The Legend of Zelda:
Breath of the Wild</longname_de>
        </menu>
        "#;
        assert_eq!(
            parse_cemu_meta_xml(xml).as_deref(),
            Some("The Legend of Zelda: Breath of the Wild")
        );
    }

    #[test]
    fn test_parse_cemu_settings_xml_game_paths() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <content>
            <GamePaths>
                <Entry>D:\WiiU\Games</Entry>
                <Entry>E:\Emulators\Roms &amp; ISOs\WiiU</Entry>
            </GamePaths>
        </content>
        "#;
        let paths = parse_cemu_settings_xml(xml);
        assert_eq!(
            paths,
            vec![
                PathBuf::from(r"D:\WiiU\Games"),
                PathBuf::from(r"E:\Emulators\Roms & ISOs\WiiU"),
            ]
        );
    }

    #[test]
    fn test_scan_cemu_folder_wua_and_unpacked_rpx_skips_updates() {
        let tmp = TempDir::new().expect("tempdir");
        let roms = tmp.path().join("WiiU");

        let botw_dir = roms.join("BotW");
        fs::create_dir_all(&botw_dir).expect("botw dir");
        fs::write(
            botw_dir.join("Zelda Breath of the Wild (Europe).wua"),
            b"wua",
        )
        .expect("write wua");
        fs::write(botw_dir.join("cover.png"), b"png").expect("write cover");

        let mk8_root = roms.join("MK8_Folder");
        fs::create_dir_all(mk8_root.join("code")).expect("code");
        fs::create_dir_all(mk8_root.join("meta")).expect("meta");
        fs::write(mk8_root.join("code").join("Turbo.rpx"), b"rpx").expect("write rpx");
        fs::write(
            mk8_root.join("meta").join("meta.xml"),
            "<menu><longname_de>Mario Kart 8</longname_de></menu>",
        )
        .expect("write meta");

        let update_code = roms
            .join("mlc01")
            .join("usr")
            .join("title")
            .join("0005000e")
            .join("1010ed00")
            .join("code");
        fs::create_dir_all(&update_code).expect("update code");
        fs::write(update_code.join("Turbo.rpx"), b"update").expect("write update rpx");

        let found = scan_emulator_folder(
            "cemu",
            Some(r"C:\Emulators\Cemu\Cemu.exe"),
            &roms,
            true,
            &ExistingGameKeys::default(),
        )
        .expect("scan cemu");

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].title, "Mario Kart 8");
        assert_eq!(found[1].title, "Zelda Breath of the Wild");
        assert!(found[1].cover_path.is_some());
        assert!(found[0]
            .launch_args
            .as_deref()
            .is_some_and(|args| args.starts_with("-f -g \"")));
        assert_eq!(found[0].tags, vec!["Cemu", "Wii U"]);
    }

    #[test]
    fn test_scan_pc_games_folder_filters_junk_and_handles_unreal_structure() {
        let tmp = TempDir::new().expect("tempdir");
        let games_root = tmp.path().join("Games");

        let ue_game = games_root.join("Expedition 33");
        let win64 = ue_game.join("Sandfall").join("Binaries").join("Win64");
        fs::create_dir_all(&win64).expect("win64");
        fs::write(ue_game.join("unins000.exe"), b"uninst").expect("unins");
        fs::write(ue_game.join("UEPrereqSetup_x64.exe"), b"prereq").expect("prereq");
        fs::write(win64.join("Sandfall-Win64-Shipping.exe"), vec![0u8; 4096])
            .expect("shipping exe");
        fs::write(ue_game.join("cover.jpg"), b"jpg").expect("cover");

        let found = scan_pc_games_folder(&games_root, &ExistingGameKeys::default())
            .expect("scan pc folder");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Expedition 33");
        assert!(found[0]
            .executable_path
            .as_deref()
            .is_some_and(|p| p.ends_with("Sandfall-Win64-Shipping.exe")));
        assert!(found[0].cover_path.is_some());
    }

    #[test]
    fn test_parse_epic_manifest_json() {
        let item = r#"{
            "bIsIncompleteInstall": false,
            "AppName": "Sugar",
            "DisplayName": "Alan Wake 2",
            "InstallLocation": "D:\\EpicGames\\AlanWake2",
            "LaunchExecutable": "AlanWake2.exe",
            "LaunchCommand": "-skipintro"
        }"#;

        let parsed = parse_epic_manifest(item).expect("epic manifest");
        assert_eq!(parsed.title, "Alan Wake 2");
        assert_eq!(parsed.source, "Epic Games");
        assert_eq!(parsed.launch_args.as_deref(), Some("-skipintro"));
        assert_eq!(parsed.tags, vec!["Epic Games"]);

        let incomplete = r#"{"bIsIncompleteInstall": true, "DisplayName": "Foo", "InstallLocation": "D:\\x", "LaunchExecutable": "x.exe"}"#;
        assert!(parse_epic_manifest(incomplete).is_none());
    }

    #[test]
    fn test_inspect_rom_file_detects_cemu_preset() {
        let tmp = TempDir::new().expect("tempdir");
        let rom = tmp.path().join("Xenoblade Chronicles X (Europe).wua");
        fs::write(&rom, b"wua").expect("write rom");

        let info = inspect_game_file(&rom).expect("inspect rom");
        assert!(info.is_rom_file);
        assert!(!info.is_emulator_binary);
        assert_eq!(info.platform, Platform::Emulator);
        assert_eq!(info.detected_emulator_id.as_deref(), Some("cemu"));
        assert_eq!(info.title, "Xenoblade Chronicles X");
        assert_eq!(info.tags, vec!["Cemu", "Wii U"]);
        assert!(info
            .launch_args
            .as_deref()
            .is_some_and(|a| a.starts_with("-f -g \"")));
    }
}
