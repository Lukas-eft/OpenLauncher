use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use eframe::egui;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{ActiveGameStatus, DesktopEntryPreview, Game, Platform, RunningGameEvent};

const FIELD_CODES: &[&str] = &[
    "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k", "%v", "%m",
];

#[derive(Debug, Clone)]
pub enum TrackerEvent {
    Started(ActiveGameStatus),
    Stopped(RunningGameEvent),
}

fn emit_tracker_event(
    event_tx: &Sender<TrackerEvent>,
    repaint_ctx: Option<&egui::Context>,
    event: TrackerEvent,
) {
    let _ = event_tx.send(event);
    if let Some(ctx) = repaint_ctx {
        ctx.request_repaint();
    }
}

pub struct PlaytimeTracker {
    active: Arc<Mutex<HashMap<String, ActiveGameStatus>>>,
}

impl Default for PlaytimeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaytimeTracker {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn is_running(&self, game_id: &str) -> bool {
        self.active
            .lock()
            .map(|map| map.contains_key(game_id))
            .unwrap_or(false)
    }

    pub fn running_games(&self) -> Vec<ActiveGameStatus> {
        self.active
            .lock()
            .map(|map| map.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn launch_game(
        &self,
        game: Game,
        db: Arc<Database>,
        event_tx: Sender<TrackerEvent>,
        repaint_ctx: Option<egui::Context>,
    ) -> AppResult<ActiveGameStatus> {
        let reserved = ActiveGameStatus {
            game_id: game.id.clone(),
            title: game.title.clone(),
            started_at: Utc::now().to_rfc3339(),
            pid: None,
        };
        self.insert_active(reserved)?;

        let result = if game.platform == Platform::Steam && game.steam_app_id.is_some() {
            let raw_app_id = game.steam_app_id.clone().unwrap_or_default();
            self.launch_steam_game(game.clone(), &raw_app_id, db, event_tx, repaint_ctx)
        } else {
            self.launch_local_game(game.clone(), db, event_tx, repaint_ctx)
        };

        if result.is_err() {
            self.remove_active(&game.id);
        }
        result
    }

    fn remove_active(&self, game_id: &str) {
        if let Ok(mut guard) = self.active.lock() {
            guard.remove(game_id);
        }
    }

    fn update_active_pid(&self, game_id: &str, pid: Option<u32>) {
        if let Ok(mut guard) = self.active.lock() {
            if let Some(status) = guard.get_mut(game_id) {
                status.pid = pid;
            }
        }
    }

    fn launch_steam_game(
        &self,
        game: Game,
        raw_app_id: &str,
        db: Arc<Database>,
        event_tx: Sender<TrackerEvent>,
        repaint_ctx: Option<egui::Context>,
    ) -> AppResult<ActiveGameStatus> {
        let app_id = validate_steam_app_id(raw_app_id)?;
        let custom_steam_path = db
            .get_settings()
            .ok()
            .and_then(|settings| settings.custom_steam_path);
        let steam_roots = crate::steam::discover_steam_roots(
            custom_steam_path.as_deref().map(Path::new),
        );

        if let Some(steam_exe) = crate::steam::find_steam_executable(&steam_roots) {
            Command::new(&steam_exe)
                .args(["-applaunch", app_id])
                .spawn()
                .map_err(|err| {
                    AppError::Launch(format!(
                        "Failed to start Steam client '{}': {err}",
                        steam_exe.display()
                    ))
                })?;
        } else {
            let uri = format!("steam://rungameid/{app_id}");
            open::that_detached(&uri).map_err(|err| {
                AppError::Launch(format!(
                    "Failed to open Steam URI '{uri}': {err}"
                ))
            })?;
        }

        let initial_started_at = Utc::now().to_rfc3339();
        let status = ActiveGameStatus {
            game_id: game.id.clone(),
            title: game.title.clone(),
            started_at: initial_started_at,
            pid: None,
        };

        emit_tracker_event(
            &event_tx,
            repaint_ctx.as_ref(),
            TrackerEvent::Started(status.clone()),
        );

        let active_map = Arc::clone(&self.active);
        thread::spawn(move || {
            monitor_steam_game(game, db, event_tx, repaint_ctx, active_map);
        });

        Ok(status)
    }

    fn launch_local_game(
        &self,
        mut game: Game,
        db: Arc<Database>,
        event_tx: Sender<TrackerEvent>,
        repaint_ctx: Option<egui::Context>,
    ) -> AppResult<ActiveGameStatus> {
        if let Some((epic_uri, epic_install_dir)) = find_epic_launch_uri(&game) {
            open::that_detached(&epic_uri).map_err(|err| {
                AppError::Launch(format!(
                    "Failed to launch Epic Games URI '{epic_uri}': {err}"
                ))
            })?;

            if game.install_dir.is_none() {
                game.install_dir = epic_install_dir;
            }

            let initial_started_at = Utc::now().to_rfc3339();
            let status = ActiveGameStatus {
                game_id: game.id.clone(),
                title: game.title.clone(),
                started_at: initial_started_at,
                pid: None,
            };

            emit_tracker_event(
                &event_tx,
                repaint_ctx.as_ref(),
                TrackerEvent::Started(status.clone()),
            );

            let active_map = Arc::clone(&self.active);
            thread::spawn(move || {
                monitor_steam_game(game, db, event_tx, repaint_ctx, active_map);
            });

            return Ok(status);
        }

        if let Some(apps_uri) = game
            .launch_args
            .as_deref()
            .map(str::trim)
            .filter(|s| s.starts_with("shell:AppsFolder\\"))
            .map(str::to_string)
        {
            Command::new("explorer.exe")
                .arg(&apps_uri)
                .spawn()
                .map_err(|err| {
                    AppError::Launch(format!(
                        "Failed to launch Microsoft Store app '{apps_uri}': {err}"
                    ))
                })?;

            let initial_started_at = Utc::now().to_rfc3339();
            let status = ActiveGameStatus {
                game_id: game.id.clone(),
                title: game.title.clone(),
                started_at: initial_started_at,
                pid: None,
            };

            emit_tracker_event(
                &event_tx,
                repaint_ctx.as_ref(),
                TrackerEvent::Started(status.clone()),
            );

            let active_map = Arc::clone(&self.active);
            thread::spawn(move || {
                monitor_steam_game(game, db, event_tx, repaint_ctx, active_map);
            });

            return Ok(status);
        }

        let raw_exe = game
            .executable_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppError::Launch("No executable path configured for this game".to_string())
            })?;

        let (resolved_exe, args, desktop_wd) = resolve_executable_and_args(
            raw_exe,
            game.launch_args.as_deref(),
        )?;

        let exe_path = Path::new(&resolved_exe);
        if (exe_path.is_absolute() || exe_path.components().count() > 1) && !exe_path.is_file() {
            return Err(AppError::Launch(format!(
                "Executable file does not exist: {}",
                exe_path.display()
            )));
        }

        let explicit_wd = game
            .working_dir
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| desktop_wd.map(PathBuf::from));

        if let Some(ref wd) = explicit_wd {
            if !wd.is_dir() {
                return Err(AppError::Launch(format!(
                    "Working directory does not exist: {}",
                    wd.display()
                )));
            }
        }

        let effective_wd = explicit_wd.or_else(|| {
            exe_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty() && p.is_dir())
                .map(Path::to_path_buf)
        });

        let mut cmd = Command::new(&resolved_exe);
        cmd.args(&args);
        if let Some(ref wd) = effective_wd {
            cmd.current_dir(wd);
        }

        let child = cmd.spawn().map_err(|err| {
            AppError::Launch(format!(
                "Failed to spawn process '{}': {err}",
                resolved_exe
            ))
        })?;

        let child_pid = child.id();
        let started_at = Utc::now().to_rfc3339();
        let session_id = db.start_play_session(&game.id, &started_at)?;

        let status = ActiveGameStatus {
            game_id: game.id.clone(),
            title: game.title.clone(),
            started_at: started_at.clone(),
            pid: Some(child_pid),
        };

        self.update_active_pid(&game.id, Some(child_pid));
        emit_tracker_event(
            &event_tx,
            repaint_ctx.as_ref(),
            TrackerEvent::Started(status.clone()),
        );

        let monitor_dir = game
            .install_dir
            .as_deref()
            .map(PathBuf::from)
            .or_else(|| {
                game.working_dir
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .filter(|p| is_specific_game_dir(p));

        let active_map = Arc::clone(&self.active);
        thread::spawn(move || {
            monitor_local_process(
                game,
                session_id,
                child,
                monitor_dir,
                db,
                event_tx,
                repaint_ctx,
                active_map,
            );
        });

        Ok(status)
    }

    fn insert_active(&self, status: ActiveGameStatus) -> AppResult<()> {
        let mut guard = self.active.lock().map_err(|_| {
            AppError::Internal("Tracker mutex is poisoned".to_string())
        })?;
        if guard.contains_key(&status.game_id) {
            return Err(AppError::Launch(format!(
                "'{}' is already running",
                status.title
            )));
        }
        guard.insert(status.game_id.clone(), status);
        Ok(())
    }
}

fn find_epic_launch_uri(game: &Game) -> Option<(String, Option<String>)> {
    if let Some(args) = game.launch_args.as_deref().map(str::trim) {
        if args.starts_with("com.epicgames.launcher://") {
            return Some((args.to_string(), game.install_dir.clone()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        use serde_json::Value;

        let manifests_dir = Path::new(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
        let entries = fs::read_dir(manifests_dir).ok()?;

        let game_exe_norm = game
            .executable_path
            .as_deref()
            .map(normalize_win_path)
            .filter(|s| !s.is_empty());
        let game_dir_norm = game
            .install_dir
            .as_deref()
            .map(normalize_win_path)
            .filter(|s| !s.is_empty());

        if game_exe_norm.is_none() && game_dir_norm.is_none() {
            return None;
        }

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()).is_none_or(|e| !e.eq_ignore_ascii_case("item")) {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(val) = serde_json::from_str::<Value>(&content) else {
                continue;
            };
            if val
                .get("bIsIncompleteInstall")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            let app_name = val
                .get("AppName")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if app_name.is_empty() || app_name.starts_with("UE_") {
                continue;
            }
            let install_location = val
                .get("InstallLocation")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if install_location.is_empty() {
                continue;
            }
            let launch_exe = val
                .get("LaunchExecutable")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");

            let manifest_dir_norm = normalize_win_path(install_location);
            let manifest_exe_norm = if !launch_exe.is_empty() {
                Some(normalize_win_path(
                    &Path::new(install_location).join(launch_exe).to_string_lossy(),
                ))
            } else {
                None
            };

            let exe_matches = game_exe_norm
                .as_deref()
                .zip(manifest_exe_norm.as_deref())
                .is_some_and(|(a, b)| a == b);
            let dir_matches = game_dir_norm
                .as_deref()
                .is_some_and(|d| d == manifest_dir_norm)
                || game_exe_norm
                    .as_deref()
                    .is_some_and(|e| e.starts_with(&format!("{manifest_dir_norm}\\")));

            if exe_matches || dir_matches {
                let ns = val
                    .get("CatalogNamespace")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                let item_id = val
                    .get("CatalogItemId")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");

                let uri = if !ns.is_empty() && !item_id.is_empty() {
                    format!(
                        "com.epicgames.launcher://apps/{ns}%3A{item_id}%3A{app_name}?action=launch&silent=true"
                    )
                } else {
                    format!(
                        "com.epicgames.launcher://apps/{app_name}?action=launch&silent=true"
                    )
                };
                return Some((uri, Some(install_location.to_string())));
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn normalize_win_path(raw: &str) -> String {
    raw.trim()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn resolve_executable_and_args(
    raw_exe: &str,
    user_args: Option<&str>,
) -> AppResult<(String, Vec<String>, Option<String>)> {
    if raw_exe.to_ascii_lowercase().ends_with(".desktop") {
        let content = fs::read_to_string(raw_exe).map_err(|err| {
            AppError::Launch(format!(
                "Failed to read .desktop file '{raw_exe}': {err}"
            ))
        })?;
        let parsed = parse_desktop_entry(&content)?;
        let mut args = match parsed.launch_args {
            Some(ref desktop_args) => parse_launch_args(desktop_args)?,
            None => Vec::new(),
        };
        if let Some(extra) = user_args {
            args.extend(parse_launch_args(extra)?);
        }
        Ok((parsed.executable_path, args, parsed.working_dir))
    } else {
        let args = match user_args {
            Some(extra) => parse_launch_args(extra)?,
            None => Vec::new(),
        };
        Ok((raw_exe.to_string(), args, None))
    }
}

#[allow(clippy::too_many_arguments)]
fn monitor_local_process(
    game: Game,
    session_id: i64,
    mut child: Child,
    monitor_dir: Option<PathBuf>,
    db: Arc<Database>,
    event_tx: Sender<TrackerEvent>,
    repaint_ctx: Option<egui::Context>,
    active_map: Arc<Mutex<HashMap<String, ActiveGameStatus>>>,
) {
    let start_instant = Instant::now();
    let direct_pid = child.id();
    let mut tracked_pids: HashSet<Pid> = HashSet::from([Pid::from_u32(direct_pid)]);
    let mut sys = System::new();
    let mut direct_exited = false;
    let mut handoff_deadline: Option<Instant> = None;
    let mut exit_instant: Option<Instant> = None;

    loop {
        if !direct_exited {
            if let Ok(Some(_)) = child.try_wait() {
                direct_exited = true;
                tracked_pids.remove(&Pid::from_u32(direct_pid));
                exit_instant = Some(Instant::now());
                if monitor_dir.is_some() {
                    handoff_deadline = Some(Instant::now() + Duration::from_secs(4));
                }
            }
        }

        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );

        let processes = sys.processes();

        loop {
            let mut added = false;
            for (&pid, proc_info) in processes {
                if tracked_pids.contains(&pid) {
                    continue;
                }
                if let Some(parent_pid) = proc_info.parent() {
                    if tracked_pids.contains(&parent_pid) {
                        tracked_pids.insert(pid);
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }

        tracked_pids.retain(|pid| {
            (!direct_exited && pid.as_u32() == direct_pid) || processes.contains_key(pid)
        });

        if direct_exited && tracked_pids.is_empty() {
            if let Some(ref dir) = monitor_dir {
                for (&pid, proc_info) in processes {
                    if let Some(exe) = proc_info.exe() {
                        if path_starts_with_dir(exe, dir) {
                            tracked_pids.insert(pid);
                            exit_instant = None;
                        }
                    }
                }
            }
        }

        let in_handoff = handoff_deadline.is_some_and(|dl| Instant::now() < dl);
        if direct_exited && tracked_pids.is_empty() && !in_handoff {
            break;
        }

        thread::sleep(Duration::from_secs(2));
    }

    let elapsed = exit_instant
        .unwrap_or_else(Instant::now)
        .saturating_duration_since(start_instant);
    let duration_seconds = elapsed.as_secs() as i64;
    let ended_at = Utc::now().to_rfc3339();
    let total_playtime = db
        .finish_play_session(session_id, &game.id, &ended_at, duration_seconds)
        .unwrap_or(game.playtime_seconds);

    remove_active_and_emit(
        &active_map,
        &event_tx,
        repaint_ctx.as_ref(),
        RunningGameEvent {
            game_id: game.id,
            session_id,
            duration_seconds,
            total_playtime_seconds: total_playtime,
            last_played_at: ended_at,
        },
    );
}

fn monitor_steam_game(
    game: Game,
    db: Arc<Database>,
    event_tx: Sender<TrackerEvent>,
    repaint_ctx: Option<egui::Context>,
    active_map: Arc<Mutex<HashMap<String, ActiveGameStatus>>>,
) {
    let resolved_dir = game
        .install_dir
        .as_deref()
        .map(PathBuf::from)
        .filter(|p| is_specific_game_dir(p))
        .or_else(|| {
            game.working_dir
                .as_deref()
                .map(PathBuf::from)
                .filter(|p| is_specific_game_dir(p))
        })
        .or_else(|| {
            game.executable_path
                .as_deref()
                .and_then(|e| Path::new(e).parent())
                .map(Path::to_path_buf)
                .filter(|p| is_specific_game_dir(p))
        });

    let Some(install_dir) = resolved_dir else {
        let now = Utc::now().to_rfc3339();
        let _ = db.touch_last_played(&game.id, &now);
        remove_active_and_emit(
            &active_map,
            &event_tx,
            repaint_ctx.as_ref(),
            RunningGameEvent {
                game_id: game.id,
                session_id: 0,
                duration_seconds: 0,
                total_playtime_seconds: game.playtime_seconds,
                last_played_at: now,
            },
        );
        return;
    };

    let mut sys = System::new();
    let mut tracked_pids: HashSet<Pid> = HashSet::new();

    let wait_deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < wait_deadline {
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );
        for (&pid, proc_info) in sys.processes() {
            if let Some(exe) = proc_info.exe() {
                if path_starts_with_dir(exe, &install_dir) {
                    tracked_pids.insert(pid);
                }
            }
        }
        if !tracked_pids.is_empty() {
            break;
        }
        thread::sleep(Duration::from_secs(2));
    }

    if tracked_pids.is_empty() {
        let now = Utc::now().to_rfc3339();
        let _ = db.touch_last_played(&game.id, &now);
        remove_active_and_emit(
            &active_map,
            &event_tx,
            repaint_ctx.as_ref(),
            RunningGameEvent {
                game_id: game.id,
                session_id: 0,
                duration_seconds: 0,
                total_playtime_seconds: game.playtime_seconds,
                last_played_at: now,
            },
        );
        return;
    }

    let start_instant = Instant::now();
    let started_at = Utc::now().to_rfc3339();
    let session_id = db.start_play_session(&game.id, &started_at).unwrap_or(0);

    loop {
        thread::sleep(Duration::from_secs(3));
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );

        let processes = sys.processes();

        for (&pid, proc_info) in processes {
            if tracked_pids.contains(&pid) {
                continue;
            }
            let in_dir = proc_info
                .exe()
                .is_some_and(|exe| path_starts_with_dir(exe, &install_dir));
            let is_child = proc_info
                .parent()
                .is_some_and(|parent| tracked_pids.contains(&parent));
            if in_dir || is_child {
                tracked_pids.insert(pid);
            }
        }

        tracked_pids.retain(|pid| processes.contains_key(pid));
        if tracked_pids.is_empty() {
            break;
        }
    }

    let elapsed_seconds = start_instant.elapsed().as_secs() as i64;
    let ended_at = Utc::now().to_rfc3339();
    let (reported_duration, total_playtime) = if session_id > 0 {
        match db.finish_play_session(session_id, &game.id, &ended_at, elapsed_seconds) {
            Ok(total) => (elapsed_seconds, total),
            Err(_) => (0, game.playtime_seconds),
        }
    } else {
        let _ = db.touch_last_played(&game.id, &ended_at);
        (0, game.playtime_seconds)
    };

    remove_active_and_emit(
        &active_map,
        &event_tx,
        repaint_ctx.as_ref(),
        RunningGameEvent {
            game_id: game.id,
            session_id,
            duration_seconds: reported_duration,
            total_playtime_seconds: total_playtime,
            last_played_at: ended_at,
        },
    );
}

fn remove_active_and_emit(
    active_map: &Arc<Mutex<HashMap<String, ActiveGameStatus>>>,
    event_tx: &Sender<TrackerEvent>,
    repaint_ctx: Option<&egui::Context>,
    event: RunningGameEvent,
) {
    if let Ok(mut guard) = active_map.lock() {
        guard.remove(&event.game_id);
    }
    emit_tracker_event(event_tx, repaint_ctx, TrackerEvent::Stopped(event));
}

fn path_starts_with_dir(exe: &Path, dir: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let exe_norm = exe.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
        let mut dir_norm = dir
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase();
        dir_norm.push('\\');
        exe_norm.starts_with(&dir_norm)
    }
    #[cfg(not(target_os = "windows"))]
    {
        exe.starts_with(dir)
    }
}

fn is_specific_game_dir(dir: &Path) -> bool {
    let raw = dir.to_string_lossy();
    let trimmed = raw.trim().trim_end_matches(['/', '\\']);
    if trimmed.is_empty() || trimmed == "/" || trimmed.len() <= 3 {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    !matches!(
        lower.as_str(),
        "/usr"
            | "/usr/bin"
            | "/usr/local/bin"
            | "/bin"
            | "/opt"
            | "c:\\windows"
            | "c:\\windows\\system32"
            | "c:\\program files"
            | "c:\\program files (x86)"
    )
}

pub fn validate_steam_app_id(app_id: &str) -> AppResult<&str> {
    let trimmed = app_id.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::InvalidInput(format!(
            "Invalid Steam App ID: '{app_id}'"
        )));
    }
    Ok(trimmed)
}

pub fn parse_launch_args(input: &str) -> AppResult<Vec<String>> {
    let chars: Vec<char> = input.chars().collect();
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current.push(ch);
            }
            i += 1;
        } else if in_double {
            if ch == '"' {
                in_double = false;
                i += 1;
            } else if ch == '\\' {
                if chars.get(i + 1) == Some(&'"') {
                    current.push('"');
                    i += 2;
                } else if chars.get(i + 1) == Some(&'\\') && chars.get(i + 2) == Some(&'"') {
                    current.push('\\');
                    i += 2;
                } else {
                    current.push('\\');
                    i += 1;
                }
            } else {
                current.push(ch);
                i += 1;
            }
        } else if ch.is_ascii_whitespace() {
            if in_token {
                args.push(std::mem::take(&mut current));
                in_token = false;
            }
            i += 1;
        } else if ch == '\'' {
            in_single = true;
            in_token = true;
            i += 1;
        } else if ch == '"' {
            in_double = true;
            in_token = true;
            i += 1;
        } else {
            current.push(ch);
            in_token = true;
            i += 1;
        }
    }

    if in_single || in_double {
        return Err(AppError::InvalidInput(
            "Unterminated quotes in launch arguments".to_string(),
        ));
    }

    if in_token {
        args.push(current);
    }

    Ok(args)
}

pub fn parse_desktop_entry(content: &str) -> AppResult<DesktopEntryPreview> {
    let mut in_desktop_entry = false;
    let mut name: Option<String> = None;
    let mut exec_raw: Option<String> = None;
    let mut working_dir: Option<String> = None;
    let mut icon: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim();
        match key {
            "Name" if name.is_none() && !val.is_empty() => name = Some(val.to_string()),
            "Exec" if exec_raw.is_none() && !val.is_empty() => exec_raw = Some(val.to_string()),
            "Path" if working_dir.is_none() && !val.is_empty() => {
                working_dir = Some(val.to_string())
            }
            "Icon" if icon.is_none() && !val.is_empty() => icon = Some(val.to_string()),
            _ => {}
        }
    }

    let exec_str = exec_raw.ok_or_else(|| {
        AppError::InvalidInput("The .desktop file does not contain an 'Exec=' entry".to_string())
    })?;

    let mut tokens = parse_launch_args(&exec_str)?;
    if tokens.is_empty() {
        return Err(AppError::InvalidInput(
            "The 'Exec=' entry in the .desktop file is empty".to_string(),
        ));
    }

    let executable_path = tokens.remove(0);
    let filtered_args: Vec<String> = tokens
        .into_iter()
        .filter(|arg| !FIELD_CODES.contains(&arg.as_str()))
        .map(|arg| quote_arg_if_needed(&arg))
        .collect();

    let launch_args = if filtered_args.is_empty() {
        None
    } else {
        Some(filtered_args.join(" "))
    };

    let title = name.unwrap_or_else(|| {
        Path::new(&executable_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Game")
            .to_string()
    });

    Ok(DesktopEntryPreview {
        title,
        executable_path,
        launch_args,
        working_dir,
        icon,
    })
}

fn quote_arg_if_needed(arg: &str) -> String {
    if arg.is_empty()
        || arg
            .chars()
            .any(|c| c.is_ascii_whitespace() || c == '"' || c == '\'')
    {
        let mut escaped = arg.replace('"', "\\\"");
        if escaped.ends_with('\\') {
            escaped.push('\\');
        }
        format!("\"{escaped}\"")
    } else {
        arg.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_launch_args_basic_and_quotes() {
        let parsed = parse_launch_args(r#"--fullscreen -w "1920 1080" 'single arg' --empty """#)
            .expect("parse args");
        assert_eq!(
            parsed,
            vec!["--fullscreen", "-w", "1920 1080", "single arg", "--empty", ""]
        );

        let escaped = parse_launch_args(r#"--label="Hello \"World\"""#).expect("escaped quotes");
        assert_eq!(escaped, vec![r#"--label=Hello "World""#]);
    }

    #[test]
    fn test_parse_launch_args_preserves_windows_paths() {
        let parsed =
            parse_launch_args(r#"--config "C:\Games\My Config\settings.ini" --dir "D:\ROMs\\""#)
                .expect("windows paths");
        assert_eq!(
            parsed,
            vec![
                "--config",
                r"C:\Games\My Config\settings.ini",
                "--dir",
                r"D:\ROMs\"
            ]
        );
    }

    #[test]
    fn test_parse_launch_args_blocks_shell_injection_as_literals() {
        let parsed = parse_launch_args("--safe; rm -rf / && $(whoami) `id` | cat")
            .expect("literal tokens");
        assert_eq!(
            parsed,
            vec!["--safe;", "rm", "-rf", "/", "&&", "$(whoami)", "`id`", "|", "cat"]
        );
    }

    #[test]
    fn test_parse_launch_args_unterminated_quote_error() {
        assert!(matches!(
            parse_launch_args(r#"--rom "C:\Games\Zelda.nsp"#),
            Err(AppError::InvalidInput(_))
        ));
        assert!(matches!(
            parse_launch_args("--flag 'unterminated"),
            Err(AppError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_validate_steam_app_id() {
        assert_eq!(validate_steam_app_id(" 1145360 ").expect("valid"), "1145360");
        assert!(validate_steam_app_id("").is_err());
        assert!(validate_steam_app_id("1145360; calc").is_err());
        assert!(validate_steam_app_id("abc").is_err());
    }

    #[test]
    fn test_parse_desktop_entry() {
        let desktop = r#"
[Desktop Entry]
Type=Application
Name=Super Mario Odyssey (Ryujinx)
Exec="/opt/ryujinx/Ryujinx" --fullscreen "/home/user/ROMs/Mario Odyssey.xci" %U
Path=/opt/ryujinx
Icon=ryujinx
"#;
        let preview = parse_desktop_entry(desktop).expect("parse desktop");
        assert_eq!(preview.title, "Super Mario Odyssey (Ryujinx)");
        assert_eq!(preview.executable_path, "/opt/ryujinx/Ryujinx");
        assert_eq!(
            preview.launch_args.as_deref(),
            Some(r#"--fullscreen "/home/user/ROMs/Mario Odyssey.xci""#)
        );
        assert_eq!(preview.working_dir.as_deref(), Some("/opt/ryujinx"));
        assert_eq!(preview.icon.as_deref(), Some("ryujinx"));
    }
}
