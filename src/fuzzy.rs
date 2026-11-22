use chrono::DateTime;

use crate::format::platform_label;
use crate::models::Game;

fn is_word_boundary(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ':' | '_' | '-')
}

pub fn parse_rfc3339_millis(iso: Option<&str>) -> Option<i64> {
    iso.and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.timestamp_millis())
}

pub fn fuzzy_score(query: &str, target: &str) -> i32 {
    let q_trimmed = query.trim();
    if q_trimmed.is_empty() {
        return 0;
    }

    let q_chars: Vec<char> = q_trimmed.to_lowercase().chars().collect();
    let t_chars: Vec<char> = target.to_lowercase().chars().collect();
    if q_chars.is_empty() || t_chars.is_empty() {
        return -1;
    }

    if q_chars.len() <= t_chars.len() {
        if let Some(exact_idx) = t_chars
            .windows(q_chars.len())
            .position(|window| window == q_chars.as_slice())
        {
            let boundary_bonus = if exact_idx == 0 || is_word_boundary(t_chars[exact_idx - 1]) {
                45
            } else {
                0
            };
            return (120 + boundary_bonus - exact_idx as i32).max(1);
        }
    }

    let mut qi = 0usize;
    let mut score = 0i32;
    let mut consecutive = 0i32;

    for (ti, &ch) in t_chars.iter().enumerate() {
        if qi >= q_chars.len() {
            break;
        }
        if ch == q_chars[qi] {
            consecutive += 1;
            let boundary = ti == 0 || is_word_boundary(t_chars[ti - 1]);
            score += 10 + consecutive * 6 + if boundary { 14 } else { 0 };
            qi += 1;
        } else {
            consecutive = 0;
            score -= 1;
        }
    }

    if qi < q_chars.len() {
        return -1;
    }

    score.max(1)
}

pub fn search_games<'a>(games: &'a [Game], query: &str) -> Vec<&'a Game> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        let mut sorted: Vec<&'a Game> = games.iter().collect();
        sorted.sort_by(|a, b| {
            b.is_favorite
                .cmp(&a.is_favorite)
                .then_with(|| {
                    let a_ts = parse_rfc3339_millis(a.last_played_at.as_deref()).unwrap_or(0);
                    let b_ts = parse_rfc3339_millis(b.last_played_at.as_deref()).unwrap_or(0);
                    b_ts.cmp(&a_ts)
                })
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        return sorted;
    }

    let mut scored: Vec<(&'a Game, i32)> = Vec::new();
    for game in games {
        let title_score = fuzzy_score(trimmed, &game.title);
        let tag_score = game
            .tags
            .iter()
            .map(|tag| fuzzy_score(trimmed, tag))
            .max()
            .unwrap_or(-1);
        let plat_score = fuzzy_score(trimmed, platform_label(game.platform));

        let best = [
            if title_score >= 0 {
                title_score * 2
            } else {
                -1
            },
            if tag_score >= 0 { tag_score + 15 } else { -1 },
            plat_score,
        ]
        .into_iter()
        .max()
        .unwrap_or(-1);

        if best >= 0 {
            scored.push((game, best));
        }
    }

    scored.sort_by(|(a, a_score), (b, b_score)| {
        b_score
            .cmp(a_score)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });

    scored.into_iter().map(|(game, _)| game).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Platform;

    fn mock_game(id: &str, title: &str, platform: Platform, fav: bool, tags: &[&str]) -> Game {
        Game {
            id: id.to_string(),
            title: title.to_string(),
            platform,
            steam_app_id: None,
            executable_path: None,
            install_dir: None,
            launch_args: None,
            working_dir: None,
            cover_url: None,
            cover_is_local: false,
            hero_url: None,
            is_favorite: fav,
            playtime_seconds: 0,
            last_played_at: None,
            added_at: "2026-11-22T10:00:00Z".to_string(),
            updated_at: "2026-11-22T10:00:00Z".to_string(),
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn test_fuzzy_score_prioritizes_prefix_and_handles_umlauts() {
        let prefix = fuzzy_score("zel", "Zelda: Breath of the Wild");
        let subseq = fuzzy_score("zbotw", "Zelda: Breath of the Wild");
        assert!(prefix > subseq);
        assert!(subseq > 0);
        assert_eq!(fuzzy_score("xyz", "Hades"), -1);
        assert!(fuzzy_score("über", "Überfall") > 0);
    }

    #[test]
    fn test_search_games_orders_favorites_and_matches_tags() {
        let games = vec![
            mock_game("1", "Celeste", Platform::Steam, false, &["Platformer"]),
            mock_game("2", "Breath of the Wild", Platform::Emulator, true, &["Cemu", "Wii U"]),
            mock_game("3", "Hades II", Platform::Manual, false, &["Roguelike"]),
        ];

        let empty_res = search_games(&games, "");
        assert_eq!(empty_res[0].id, "2");

        let tag_res = search_games(&games, "cemu");
        assert_eq!(tag_res.len(), 1);
        assert_eq!(tag_res[0].id, "2");

        let title_res = search_games(&games, "had");
        assert_eq!(title_res.len(), 1);
        assert_eq!(title_res[0].id, "3");
    }
}
