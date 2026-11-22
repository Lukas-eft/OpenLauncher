use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    AppSettings, BulkImportResult, Game, NewGameInput, Platform, PlaySession, SteamApp,
    UpdateGameInput,
};

const SETTINGS_KEY: &str = "app_settings";

#[derive(Debug, Clone, Default)]
pub struct ExistingGameKeys {
    pub steam_app_ids: HashSet<String>,
    pub exe_arg_pairs: HashSet<(String, String)>,
}

impl ExistingGameKeys {
    pub fn contains_steam(&self, app_id: &str) -> bool {
        self.steam_app_ids.contains(app_id.trim())
    }

    pub fn contains_exe_args(&self, exe: Option<&str>, args: Option<&str>) -> bool {
        normalize_exe_arg_pair(exe, args)
            .is_some_and(|pair| self.exe_arg_pairs.contains(&pair))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Inserted,
    Updated,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {

    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn lock_conn(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::Internal("Database mutex is poisoned".to_string()))
    }

    fn init_schema(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                platform TEXT NOT NULL CHECK(platform IN ('steam', 'manual', 'emulator')),
                steam_app_id TEXT UNIQUE,
                executable_path TEXT,
                install_dir TEXT,
                launch_args TEXT,
                working_dir TEXT,
                cover_url TEXT,
                cover_is_local INTEGER NOT NULL DEFAULT 0,
                hero_url TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                playtime_seconds INTEGER NOT NULL DEFAULT 0,
                last_played_at TEXT,
                added_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE
            );

            CREATE TABLE IF NOT EXISTS game_tags (
                game_id TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (game_id, tag_id),
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS play_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                duration_seconds INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_play_sessions_game_started
                ON play_sessions(game_id, started_at DESC);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn list_games(&self) -> AppResult<Vec<Game>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, platform, steam_app_id, executable_path, install_dir,
                    launch_args, working_dir, cover_url, cover_is_local, hero_url,
                    is_favorite, playtime_seconds, last_played_at, added_at, updated_at
             FROM games
             ORDER BY title COLLATE NOCASE ASC",
        )?;

        let mut games = stmt
            .query_map([], row_to_game)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut tags_stmt = conn.prepare(
            "SELECT gt.game_id, t.name
             FROM game_tags gt
             INNER JOIN tags t ON t.id = gt.tag_id
             ORDER BY t.name COLLATE NOCASE ASC",
        )?;
        let mut tags_by_game: HashMap<String, Vec<String>> = HashMap::new();
        let tag_rows = tags_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for pair in tag_rows {
            let (game_id, tag_name) = pair?;
            tags_by_game.entry(game_id).or_default().push(tag_name);
        }

        for game in &mut games {
            if let Some(tags) = tags_by_game.remove(&game.id) {
                game.tags = tags;
            }
        }

        Ok(games)
    }

    pub fn get_game(&self, id: &str) -> AppResult<Game> {
        let conn = self.lock_conn()?;
        Self::get_game_with_conn(&conn, id)
    }

    fn get_game_with_conn(conn: &Connection, id: &str) -> AppResult<Game> {
        let mut game = conn
            .query_row(
                "SELECT id, title, platform, steam_app_id, executable_path, install_dir,
                        launch_args, working_dir, cover_url, cover_is_local, hero_url,
                        is_favorite, playtime_seconds, last_played_at, added_at, updated_at
                 FROM games WHERE id = ?1",
                params![id],
                row_to_game,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("Game '{id}' not found")))?;

        let mut tags_stmt = conn.prepare(
            "SELECT t.name
             FROM game_tags gt
             INNER JOIN tags t ON t.id = gt.tag_id
             WHERE gt.game_id = ?1
             ORDER BY t.name COLLATE NOCASE ASC",
        )?;
        game.tags = tags_stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(game)
    }

    pub fn create_game(&self, input: NewGameInput) -> AppResult<Game> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        let id = insert_game_tx(&tx, &input)?;
        tx.commit()?;

        Self::get_game_with_conn(&conn, &id)
    }

    pub fn find_existing_keys(&self) -> AppResult<ExistingGameKeys> {
        let conn = self.lock_conn()?;
        find_existing_keys_with_conn(&conn)
    }

    pub fn import_games_bulk(&self, items: Vec<NewGameInput>) -> AppResult<BulkImportResult> {
        let mut conn = self.lock_conn()?;
        let mut existing = find_existing_keys_with_conn(&conn)?;
        let tx = conn.transaction()?;

        let mut added_count = 0usize;
        let mut skipped_count = 0usize;
        let mut errors = Vec::new();

        for input in items {
            if let Some(ref app_id) = input.steam_app_id {
                if !app_id.trim().is_empty() && existing.contains_steam(app_id) {
                    skipped_count += 1;
                    continue;
                }
            }
            if existing.contains_exe_args(
                input.executable_path.as_deref(),
                input.launch_args.as_deref(),
            ) {
                skipped_count += 1;
                continue;
            }

            match insert_game_tx(&tx, &input) {
                Ok(_) => {
                    if let Some(ref app_id) = input.steam_app_id {
                        let trimmed = app_id.trim();
                        if !trimmed.is_empty() {
                            existing.steam_app_ids.insert(trimmed.to_string());
                        }
                    }
                    if let Some(pair) = normalize_exe_arg_pair(
                        input.executable_path.as_deref(),
                        input.launch_args.as_deref(),
                    ) {
                        existing.exe_arg_pairs.insert(pair);
                    }
                    added_count += 1;
                }
                Err(err) => errors.push(format!("{}: {err}", input.title)),
            }
        }

        tx.commit()?;

        Ok(BulkImportResult {
            added_count,
            skipped_count,
            errors,
        })
    }

    pub fn update_game(&self, input: UpdateGameInput) -> AppResult<Game> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::InvalidInput(
                "Game title cannot be empty".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let tags = normalize_tags(&input.tags);

        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        let affected = tx.execute(
            "UPDATE games
             SET title = ?1,
                 platform = ?2,
                 steam_app_id = ?3,
                 executable_path = ?4,
                 install_dir = ?5,
                 launch_args = ?6,
                 working_dir = ?7,
                 cover_url = ?8,
                 cover_is_local = ?9,
                 hero_url = ?10,
                 is_favorite = ?11,
                 updated_at = ?12
             WHERE id = ?13",
            params![
                title,
                input.platform.as_str(),
                normalize_opt(input.steam_app_id),
                normalize_opt(input.executable_path),
                normalize_opt(input.install_dir),
                normalize_opt(input.launch_args),
                normalize_opt(input.working_dir),
                normalize_opt(input.cover_url),
                i32::from(input.cover_is_local),
                normalize_opt(input.hero_url),
                i32::from(input.is_favorite),
                now,
                input.id,
            ],
        )?;

        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Game '{}' not found",
                input.id
            )));
        }

        sync_game_tags(&tx, &input.id, &tags)?;
        tx.commit()?;

        Self::get_game_with_conn(&conn, &input.id)
    }

    pub fn delete_game(&self, id: &str) -> AppResult<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        let affected = tx.execute("DELETE FROM games WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Game '{id}' not found")));
        }
        prune_orphan_tags(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn toggle_favorite(&self, id: &str) -> AppResult<Game> {
        let now = Utc::now().to_rfc3339();
        let conn = self.lock_conn()?;
        let affected = conn.execute(
            "UPDATE games
             SET is_favorite = CASE WHEN is_favorite = 0 THEN 1 ELSE 0 END,
                 updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("Game '{id}' not found")));
        }
        Self::get_game_with_conn(&conn, id)
    }

    pub fn list_all_tags(&self) -> AppResult<Vec<String>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name COLLATE NOCASE ASC")?;
        let tags = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    pub fn upsert_steam_game(&self, app: &SteamApp) -> AppResult<UpsertOutcome> {
        let conn = self.lock_conn()?;
        let existing: Option<(String, i32)> = conn
            .query_row(
                "SELECT id, cover_is_local FROM games WHERE steam_app_id = ?1",
                params![app.app_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let now = Utc::now().to_rfc3339();
        let install_dir = app.install_dir.to_string_lossy().into_owned();

        if let Some((existing_id, cover_is_local)) = existing {
            if cover_is_local != 0 {
                conn.execute(
                    "UPDATE games
                     SET title = ?1, install_dir = ?2, hero_url = ?3, updated_at = ?4
                     WHERE id = ?5",
                    params![app.name, install_dir, app.hero_url, now, existing_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE games
                     SET title = ?1, install_dir = ?2, cover_url = ?3, hero_url = ?4, updated_at = ?5
                     WHERE id = ?6",
                    params![
                        app.name,
                        install_dir,
                        app.cover_url,
                        app.hero_url,
                        now,
                        existing_id
                    ],
                )?;
            }
            Ok(UpsertOutcome::Updated)
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO games (
                    id, title, platform, steam_app_id, executable_path, install_dir,
                    launch_args, working_dir, cover_url, cover_is_local, hero_url,
                    is_favorite, playtime_seconds, last_played_at, added_at, updated_at
                ) VALUES (?1, ?2, 'steam', ?3, NULL, ?4, NULL, NULL, ?5, 0, ?6, 0, 0, NULL, ?7, ?7)",
                params![
                    id,
                    app.name,
                    app.app_id,
                    install_dir,
                    app.cover_url,
                    app.hero_url,
                    now
                ],
            )?;
            Ok(UpsertOutcome::Inserted)
        }
    }

    pub fn start_play_session(&self, game_id: &str, started_at: &str) -> AppResult<i64> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO play_sessions (game_id, started_at, ended_at, duration_seconds)
             VALUES (?1, ?2, NULL, 0)",
            params![game_id, started_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn finish_play_session(
        &self,
        session_id: i64,
        game_id: &str,
        ended_at: &str,
        duration_seconds: i64,
    ) -> AppResult<i64> {
        let clamped_duration = duration_seconds.max(0);
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE play_sessions
             SET ended_at = ?1, duration_seconds = ?2
             WHERE id = ?3 AND game_id = ?4",
            params![ended_at, clamped_duration, session_id, game_id],
        )?;

        tx.execute(
            "UPDATE games
             SET playtime_seconds = playtime_seconds + ?1,
                 last_played_at = ?2,
                 updated_at = ?2
             WHERE id = ?3",
            params![clamped_duration, ended_at, game_id],
        )?;

        let total: i64 = tx.query_row(
            "SELECT playtime_seconds FROM games WHERE id = ?1",
            params![game_id],
            |row| row.get(0),
        )?;

        tx.commit()?;
        Ok(total)
    }

    pub fn touch_last_played(&self, game_id: &str, played_at: &str) -> AppResult<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE games SET last_played_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![played_at, game_id],
        )?;
        Ok(())
    }

    pub fn get_game_sessions(&self, game_id: &str, limit: usize) -> AppResult<Vec<PlaySession>> {
        let conn = self.lock_conn()?;
        let bounded = limit.clamp(1, 500) as i64;
        let mut stmt = conn.prepare(
            "SELECT id, game_id, started_at, ended_at, duration_seconds
             FROM play_sessions
             WHERE game_id = ?1
             ORDER BY started_at DESC, id DESC
             LIMIT ?2",
        )?;
        let sessions = stmt
            .query_map(params![game_id, bounded], |row| {
                Ok(PlaySession {
                    id: row.get(0)?,
                    game_id: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    duration_seconds: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let conn = self.lock_conn()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![SETTINGS_KEY],
                |row| row.get(0),
            )
            .optional()?;

        match raw {
            Some(json) => serde_json::from_str::<AppSettings>(&json).map_err(|err| {
                AppError::Internal(format!("Failed to parse settings: {err}"))
            }),
            None => Ok(AppSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let normalized = AppSettings {
            minimize_to_tray: settings.minimize_to_tray,
            custom_steam_path: normalize_opt(settings.custom_steam_path.clone()),
            auto_scan_steam_on_startup: settings.auto_scan_steam_on_startup,
        };
        let json = serde_json::to_string(&normalized).map_err(|err| {
            AppError::Internal(format!(
                "Failed to serialize settings: {err}"
            ))
        })?;
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SETTINGS_KEY, json],
        )?;
        Ok(())
    }
}

fn insert_game_tx(tx: &Transaction<'_>, input: &NewGameInput) -> AppResult<String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::InvalidInput(
            "Game title cannot be empty".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let tags = normalize_tags(&input.tags);

    tx.execute(
        "INSERT INTO games (
            id, title, platform, steam_app_id, executable_path, install_dir,
            launch_args, working_dir, cover_url, cover_is_local, hero_url,
            is_favorite, playtime_seconds, last_played_at, added_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, NULL, ?13, ?13)",
        params![
            id,
            title,
            input.platform.as_str(),
            normalize_opt(input.steam_app_id.clone()),
            normalize_opt(input.executable_path.clone()),
            normalize_opt(input.install_dir.clone()),
            normalize_opt(input.launch_args.clone()),
            normalize_opt(input.working_dir.clone()),
            normalize_opt(input.cover_url.clone()),
            i32::from(input.cover_is_local),
            normalize_opt(input.hero_url.clone()),
            i32::from(input.is_favorite),
            now,
        ],
    )?;

    sync_game_tags(tx, &id, &tags)?;
    Ok(id)
}

fn find_existing_keys_with_conn(conn: &Connection) -> AppResult<ExistingGameKeys> {
    let mut stmt =
        conn.prepare("SELECT steam_app_id, executable_path, launch_args FROM games")?;
    let mut keys = ExistingGameKeys::default();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    for row in rows {
        let (steam_app_id, executable_path, launch_args) = row?;
        if let Some(app_id) = steam_app_id {
            let trimmed = app_id.trim();
            if !trimmed.is_empty() {
                keys.steam_app_ids.insert(trimmed.to_string());
            }
        }
        if let Some(pair) =
            normalize_exe_arg_pair(executable_path.as_deref(), launch_args.as_deref())
        {
            keys.exe_arg_pairs.insert(pair);
        }
    }

    Ok(keys)
}

fn normalize_exe_arg_pair(exe: Option<&str>, args: Option<&str>) -> Option<(String, String)> {
    let exe_trimmed = exe?.trim();
    if exe_trimmed.is_empty() {
        return None;
    }
    #[cfg(target_os = "windows")]
    let exe_norm = exe_trimmed.replace('/', "\\").to_ascii_lowercase();
    #[cfg(not(target_os = "windows"))]
    let exe_norm = exe_trimmed.to_string();

    let args_norm = args.unwrap_or("").trim().to_ascii_lowercase();
    Some((exe_norm, args_norm))
}

fn row_to_game(row: &Row<'_>) -> rusqlite::Result<Game> {
    let platform_raw: String = row.get(2)?;
    let platform = Platform::from_str(&platform_raw).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid platform: {platform_raw}"),
            )),
        )
    })?;

    Ok(Game {
        id: row.get(0)?,
        title: row.get(1)?,
        platform,
        steam_app_id: row.get(3)?,
        executable_path: row.get(4)?,
        install_dir: row.get(5)?,
        launch_args: row.get(6)?,
        working_dir: row.get(7)?,
        cover_url: row.get(8)?,
        cover_is_local: row.get::<_, i32>(9)? != 0,
        hero_url: row.get(10)?,
        is_favorite: row.get::<_, i32>(11)? != 0,
        playtime_seconds: row.get(12)?,
        last_played_at: row.get(13)?,
        added_at: row.get(14)?,
        updated_at: row.get(15)?,
        tags: Vec::new(),
    })
}

fn normalize_opt(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in tags {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if seen.insert(lower) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn sync_game_tags(tx: &Transaction<'_>, game_id: &str, tags: &[String]) -> AppResult<()> {
    tx.execute("DELETE FROM game_tags WHERE game_id = ?1", params![game_id])?;
    for tag in tags {
        tx.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])?;
        let tag_id: i64 = tx.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![tag],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO game_tags (game_id, tag_id) VALUES (?1, ?2)",
            params![game_id, tag_id],
        )?;
    }
    prune_orphan_tags(tx)?;
    Ok(())
}

fn prune_orphan_tags(tx: &Transaction<'_>) -> AppResult<()> {
    tx.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM game_tags)",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn sample_manual_input(title: &str) -> NewGameInput {
        NewGameInput {
            title: title.to_string(),
            platform: Platform::Manual,
            steam_app_id: None,
            executable_path: Some("/games/hades/hades.exe".to_string()),
            install_dir: Some("/games/hades".to_string()),
            launch_args: Some("--fullscreen".to_string()),
            working_dir: Some("/games/hades".to_string()),
            cover_url: None,
            cover_is_local: false,
            hero_url: None,
            is_favorite: false,
            tags: vec![
                "Roguelike".to_string(),
                "Action".to_string(),
                "roguelike".to_string(),
                "  ".to_string(),
            ],
        }
    }

    #[test]
    fn test_crud_game_and_tags() {
        let db = Database::open_in_memory().expect("in-memory db");

        let created = db
            .create_game(sample_manual_input("  Hades II  "))
            .expect("create game");
        assert_eq!(created.title, "Hades II");
        assert_eq!(created.platform, Platform::Manual);
        assert_eq!(created.tags, vec!["Action", "Roguelike"]);

        let all_games = db.list_games().expect("list games");
        assert_eq!(all_games.len(), 1);
        assert_eq!(all_games[0].id, created.id);

        let updated = db
            .update_game(UpdateGameInput {
                id: created.id.clone(),
                title: "Hades II (Updated)".to_string(),
                platform: Platform::Emulator,
                steam_app_id: None,
                executable_path: Some("/emu/ryujinx".to_string()),
                install_dir: None,
                launch_args: Some("rom.nsp".to_string()),
                working_dir: None,
                cover_url: Some("/covers/hades.png".to_string()),
                cover_is_local: true,
                hero_url: None,
                is_favorite: true,
                tags: vec!["Indie".to_string()],
            })
            .expect("update game");

        assert_eq!(updated.title, "Hades II (Updated)");
        assert_eq!(updated.platform, Platform::Emulator);
        assert!(updated.is_favorite);
        assert!(updated.cover_is_local);
        assert_eq!(updated.tags, vec!["Indie"]);

        let all_tags = db.list_all_tags().expect("list tags");
        assert_eq!(all_tags, vec!["Indie"]);

        db.delete_game(&created.id).expect("delete game");
        assert!(db.list_games().expect("empty list").is_empty());
        assert!(db.list_all_tags().expect("empty tags").is_empty());
        assert!(matches!(
            db.delete_game(&created.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_toggle_favorite() {
        let db = Database::open_in_memory().expect("in-memory db");
        let game = db
            .create_game(sample_manual_input("Celeste"))
            .expect("create game");
        assert!(!game.is_favorite);

        let fav = db.toggle_favorite(&game.id).expect("toggle on");
        assert!(fav.is_favorite);

        let unfav = db.toggle_favorite(&game.id).expect("toggle off");
        assert!(!unfav.is_favorite);
    }

    #[test]
    fn test_upsert_steam_game_preserves_customizations() {
        let db = Database::open_in_memory().expect("in-memory db");
        let app = SteamApp {
            app_id: "1145360".to_string(),
            name: "Hades".to_string(),
            install_dir: PathBuf::from("C:\\Steam\\steamapps\\common\\Hades"),
            cover_url: "https://cdn.cloudflare.steamstatic.com/steam/apps/1145360/library_600x900_2x.jpg".to_string(),
            hero_url: "https://cdn.cloudflare.steamstatic.com/steam/apps/1145360/library_hero.jpg".to_string(),
        };

        assert_eq!(
            db.upsert_steam_game(&app).expect("insert"),
            UpsertOutcome::Inserted
        );

        let initial = db.list_games().expect("list")[0].clone();
        db.update_game(UpdateGameInput {
            id: initial.id.clone(),
            title: initial.title.clone(),
            platform: Platform::Steam,
            steam_app_id: initial.steam_app_id.clone(),
            executable_path: None,
            install_dir: initial.install_dir.clone(),
            launch_args: Some("-dx11".to_string()),
            working_dir: None,
            cover_url: Some("/app/covers/custom.png".to_string()),
            cover_is_local: true,
            hero_url: initial.hero_url.clone(),
            is_favorite: true,
            tags: vec!["Favorit-Tag".to_string()],
        })
        .expect("customize game");

        let sid = db
            .start_play_session(&initial.id, "2026-11-22T12:00:00Z")
            .expect("start session");
        db.finish_play_session(sid, &initial.id, "2026-11-22T12:30:00Z", 1800)
            .expect("finish session");

        let moved_app = SteamApp {
            install_dir: PathBuf::from("D:\\SteamLibrary\\steamapps\\common\\Hades"),
            ..app
        };
        assert_eq!(
            db.upsert_steam_game(&moved_app).expect("update"),
            UpsertOutcome::Updated
        );

        let after = db.get_game(&initial.id).expect("get game");
        assert_eq!(
            after.install_dir.as_deref(),
            Some("D:\\SteamLibrary\\steamapps\\common\\Hades")
        );
        assert!(after.is_favorite);
        assert_eq!(after.playtime_seconds, 1800);
        assert_eq!(after.launch_args.as_deref(), Some("-dx11"));
        assert_eq!(after.cover_url.as_deref(), Some("/app/covers/custom.png"));
        assert!(after.cover_is_local);
        assert_eq!(after.tags, vec!["Favorit-Tag"]);
    }

    #[test]
    fn test_play_sessions_and_playtime_accumulation() {
        let db = Database::open_in_memory().expect("in-memory db");
        let game = db
            .create_game(sample_manual_input("Hollow Knight"))
            .expect("create game");

        let s1 = db
            .start_play_session(&game.id, "2026-11-20T18:00:00Z")
            .expect("s1");
        let total1 = db
            .finish_play_session(s1, &game.id, "2026-11-20T19:00:00Z", 3600)
            .expect("finish s1");
        assert_eq!(total1, 3600);

        let s2 = db
            .start_play_session(&game.id, "2026-11-21T20:00:00Z")
            .expect("s2");
        let total2 = db
            .finish_play_session(s2, &game.id, "2026-11-21T20:15:30Z", 930)
            .expect("finish s2");
        assert_eq!(total2, 4530);

        let refreshed = db.get_game(&game.id).expect("get");
        assert_eq!(refreshed.playtime_seconds, 4530);
        assert_eq!(
            refreshed.last_played_at.as_deref(),
            Some("2026-11-21T20:15:30Z")
        );

        let sessions = db.get_game_sessions(&game.id, 10).expect("sessions");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, s2);
        assert_eq!(sessions[0].duration_seconds, 930);
        assert_eq!(sessions[1].id, s1);

        db.delete_game(&game.id).expect("delete");
        let remaining = db.get_game_sessions(&game.id, 10).expect("empty sessions");
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_settings_roundtrip() {
        let db = Database::open_in_memory().expect("in-memory db");
        assert_eq!(db.get_settings().expect("default"), AppSettings::default());

        let custom = AppSettings {
            minimize_to_tray: true,
            custom_steam_path: Some("  D:\\Steam  ".to_string()),
            auto_scan_steam_on_startup: true,
        };
        db.save_settings(&custom).expect("save");

        let loaded = db.get_settings().expect("load");
        assert!(loaded.minimize_to_tray);
        assert!(loaded.auto_scan_steam_on_startup);
        assert_eq!(loaded.custom_steam_path.as_deref(), Some("D:\\Steam"));
    }

    #[test]
    fn test_import_games_bulk_skips_duplicates() {
        let db = Database::open_in_memory().expect("in-memory db");
        db.create_game(NewGameInput {
            title: "Breath of the Wild".to_string(),
            platform: Platform::Emulator,
            steam_app_id: None,
            executable_path: Some(r"C:\Emulators\Cemu\Cemu.exe".to_string()),
            install_dir: None,
            launch_args: Some(r#"-f -g "D:\WiiU\BotW.wua""#.to_string()),
            working_dir: Some(r"C:\Emulators\Cemu".to_string()),
            cover_url: None,
            cover_is_local: false,
            hero_url: None,
            is_favorite: false,
            tags: vec!["Cemu".to_string(), "Wii U".to_string()],
        })
        .expect("initial game");

        let batch = vec![

            NewGameInput {
                title: "Breath of the Wild".to_string(),
                platform: Platform::Emulator,
                steam_app_id: None,
                executable_path: Some(r"c:/emulators/cemu/cemu.exe".to_string()),
                install_dir: None,
                launch_args: Some(r#"-f -g "d:\wiiu\botw.wua""#.to_string()),
                working_dir: None,
                cover_url: None,
                cover_is_local: false,
                hero_url: None,
                is_favorite: false,
                tags: vec!["Wii U".to_string()],
            },

            NewGameInput {
                title: "Mario Kart 8".to_string(),
                platform: Platform::Emulator,
                steam_app_id: None,
                executable_path: Some(r"C:\Emulators\Cemu\Cemu.exe".to_string()),
                install_dir: None,
                launch_args: Some(r#"-f -g "D:\WiiU\MK8.wua""#.to_string()),
                working_dir: Some(r"C:\Emulators\Cemu".to_string()),
                cover_url: None,
                cover_is_local: false,
                hero_url: None,
                is_favorite: false,
                tags: vec!["Cemu".to_string(), "Wii U".to_string()],
            },
        ];

        let result = db.import_games_bulk(batch).expect("bulk import");
        assert_eq!(result.added_count, 1);
        assert_eq!(result.skipped_count, 1);
        assert!(result.errors.is_empty());
        assert_eq!(db.list_games().expect("games").len(), 2);
    }
}
