use crate::keybind::KeyBind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(
    clippy::struct_excessive_bools,
    reason = "independent user-facing settings, not a state machine — a bitflags/enum refactor would hurt readability here without a real correctness benefit"
)]
pub struct Config {
    pub menu_key: KeyBind,
    pub next_skin_key: KeyBind,
    pub previous_skin_key: KeyBind,
    pub rainbow_text: bool,
    pub font_scale: f32,
    pub hero_name: bool,
    pub quick_skin_change: bool,
    pub is_open: bool,
    pub current_combo_skin_index: i32,
    pub current_combo_minion_index: i32,
    pub current_minion_skin_index: i32,
    pub current_combo_ward_index: i32,
    pub current_ward_skin_index: i32,
    pub current_combo_order_turret_index: i32,
    pub current_combo_chaos_turret_index: i32,
    pub current_combo_ally_skin_index: HashMap<u64, i32>,
    pub current_combo_enemy_skin_index: HashMap<u64, i32>,
    pub current_combo_jungle_mob_skin_index: HashMap<u64, i32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            menu_key: KeyBind::new(crate::keybind::KeyCode::Insert),
            next_skin_key: KeyBind::new(crate::keybind::KeyCode::PageUp),
            previous_skin_key: KeyBind::new(crate::keybind::KeyCode::PageDown),
            rainbow_text: false,
            font_scale: 1.0,
            hero_name: true,
            quick_skin_change: false,
            is_open: true,
            current_combo_skin_index: 0,
            current_combo_minion_index: 0,
            current_minion_skin_index: -1,
            current_combo_ward_index: 0,
            current_ward_skin_index: -1,
            current_combo_order_turret_index: 0,
            current_combo_chaos_turret_index: 0,
            current_combo_ally_skin_index: HashMap::new(),
            current_combo_enemy_skin_index: HashMap::new(),
            current_combo_jungle_mob_skin_index: HashMap::new(),
        }
    }
}

fn dump_index_map(map: &HashMap<u64, i32>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        obj.insert(k.to_string(), serde_json::json!(v));
    }
    serde_json::Value::Object(obj)
}

fn load_index_map(value: &serde_json::Value, key: &str) -> HashMap<u64, i32> {
    let mut out = HashMap::new();
    if let Some(serde_json::Value::Object(obj)) = value.get(key) {
        for (k, v) in obj {
            if let (Ok(k), Some(v)) = (k.parse::<u64>(), v.as_i64()) {
                out.insert(k, v as i32);
            }
        }
    }
    out
}

impl Config {
    pub fn save(&self, dir: &Path, current_player_model: Option<&str>) {
        let mut root: serde_json::Map<String, serde_json::Value> =
            std::fs::read_to_string(dir.join("WildSkin64"))
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();

        if let Some(model) = current_player_model {
            root.insert(
                format!("{model}.current_combo_skin_index"),
                serde_json::json!(self.current_combo_skin_index),
            );
        }

        root.insert(
            "menuKey".into(),
            serde_json::json!(self.menu_key.to_string()),
        );
        root.insert(
            "nextSkinKey".into(),
            serde_json::json!(self.next_skin_key.to_string()),
        );
        root.insert(
            "previousSkinKey".into(),
            serde_json::json!(self.previous_skin_key.to_string()),
        );
        root.insert("heroName".into(), serde_json::json!(self.hero_name));
        root.insert("raibowText".into(), serde_json::json!(self.rainbow_text)); // legacy typo, kept for back-compat
        root.insert(
            "quickSkinChange".into(),
            serde_json::json!(self.quick_skin_change),
        );
        root.insert("isOpen".into(), serde_json::json!(self.is_open));
        root.insert("fontScale".into(), serde_json::json!(self.font_scale));
        root.insert(
            "current_combo_ward_index".into(),
            serde_json::json!(self.current_combo_ward_index),
        );
        root.insert(
            "current_ward_skin_index".into(),
            serde_json::json!(self.current_ward_skin_index),
        );
        root.insert(
            "current_minion_skin_index".into(),
            serde_json::json!(self.current_minion_skin_index),
        );
        root.insert(
            "current_combo_ally_skin_index".into(),
            dump_index_map(&self.current_combo_ally_skin_index),
        );
        root.insert(
            "current_combo_enemy_skin_index".into(),
            dump_index_map(&self.current_combo_enemy_skin_index),
        );
        root.insert(
            "current_combo_jungle_mob_skin_index".into(),
            dump_index_map(&self.current_combo_jungle_mob_skin_index),
        );

        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let _ = std::fs::write(
            dir.join("WildSkin64"),
            serde_json::Value::Object(root).to_string(),
        );
    }

    pub fn load(&mut self, dir: &Path, current_player_model: Option<&str>) {
        let Ok(raw) = std::fs::read_to_string(dir.join("WildSkin64")) else {
            return;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return;
        };

        if let Some(model) = current_player_model {
            let key = format!("{model}.current_combo_skin_index");
            self.current_combo_skin_index = json
                .get(&key)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i32;
        }

        self.menu_key = KeyBind::from_name(
            json.get("menuKey")
                .and_then(|v| v.as_str())
                .unwrap_or("INSERT"),
        );
        self.next_skin_key = KeyBind::from_name(
            json.get("nextSkinKey")
                .and_then(|v| v.as_str())
                .unwrap_or("PAGE_UP"),
        );
        self.previous_skin_key = KeyBind::from_name(
            json.get("previousSkinKey")
                .and_then(|v| v.as_str())
                .unwrap_or("PAGE_DOWN"),
        );
        self.hero_name = json
            .get("heroName")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        self.rainbow_text = json
            .get("raibowText")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        self.quick_skin_change = json
            .get("quickSkinChange")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        self.is_open = json
            .get("isOpen")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        self.font_scale = json
            .get("fontScale")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.0) as f32;
        self.current_combo_ward_index = json
            .get("current_combo_ward_index")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0) as i32;
        self.current_ward_skin_index = json
            .get("current_ward_skin_index")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1) as i32;
        self.current_minion_skin_index = json
            .get("current_minion_skin_index")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1) as i32;
        self.current_combo_ally_skin_index = load_index_map(&json, "current_combo_ally_skin_index");
        self.current_combo_enemy_skin_index =
            load_index_map(&json, "current_combo_enemy_skin_index");
        self.current_combo_jungle_mob_skin_index =
            load_index_map(&json, "current_combo_jungle_mob_skin_index");
    }

    #[expect(
        dead_code,
        reason = "config reset, ported from the original for parity; no reset control in the GUI yet"
    )]
    pub fn reset(&mut self) {
        *self = Self::default();
        self.rainbow_text = true; // original's reset() sets true, unlike its own Default (false) — preserved as-is
    }
}

pub fn config_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_default()
        .join(shared::APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json_including_the_legacy_typo_key() {
        let dir = std::env::temp_dir().join(format!("wildskin_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut cfg = Config::default();
        cfg.rainbow_text = true;
        cfg.font_scale = 1.5;
        cfg.current_combo_enemy_skin_index.insert(123_456_789, 3);
        cfg.save(&dir, Some("Ahri"));

        let raw = std::fs::read_to_string(dir.join("WildSkin64")).unwrap();
        assert!(
            raw.contains("\"raibowText\":true"),
            "must keep the legacy typo'd key: {raw}"
        );

        let mut loaded = Config::default();
        loaded.load(&dir, Some("Ahri"));
        assert!(loaded.rainbow_text);
        assert_eq!(loaded.font_scale, 1.5);
        assert_eq!(
            loaded.current_combo_enemy_skin_index.get(&123_456_789),
            Some(&3)
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_keeps_defaults() {
        let dir =
            std::env::temp_dir().join(format!("wildskin_test_missing_{}", std::process::id()));
        let mut cfg = Config::default();
        cfg.load(&dir, None);
        assert_eq!(cfg.font_scale, 1.0);
        assert!(cfg.menu_key.is_set());
    }

    #[test]
    fn save_preserves_other_champions_persisted_skin_index() {
        let dir =
            std::env::temp_dir().join(format!("wildskin_test_preserve_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut cfg_a = Config::default();
        cfg_a.current_combo_skin_index = 5;
        cfg_a.save(&dir, Some("Ahri"));

        let mut cfg_b = Config::default();
        cfg_b.current_combo_skin_index = 2;
        cfg_b.save(&dir, Some("Zed"));

        let raw = std::fs::read_to_string(dir.join("WildSkin64")).unwrap();
        assert!(
            raw.contains("\"Ahri.current_combo_skin_index\":5"),
            "saving Zed must not wipe Ahri's key: {raw}"
        );
        assert!(
            raw.contains("\"Zed.current_combo_skin_index\":2"),
            "Zed's own key must be present: {raw}"
        );

        // A save with no current player model (e.g. "Other Champs" tab / Random
        // Skins) must not wipe the previously saved per-champion keys either.
        Config::default().save(&dir, None);
        let raw = std::fs::read_to_string(dir.join("WildSkin64")).unwrap();
        assert!(
            raw.contains("\"Ahri.current_combo_skin_index\":5"),
            "save(None) must not wipe Ahri's key: {raw}"
        );
        assert!(
            raw.contains("\"Zed.current_combo_skin_index\":2"),
            "save(None) must not wipe Zed's key: {raw}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
