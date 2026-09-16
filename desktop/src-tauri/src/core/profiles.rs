//! G6 profile infrastructure: typed schema + atomic persistence.
//! ponytail: no IPC/CLI wiring yet; that's T7 after D09 approval.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Camera profile: persisted user preferences for a capture session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraProfile {
    pub name: String,
    pub resolution: Resolution,
    pub fps: u32,
    pub camera_id: String,
    #[serde(default)]
    pub controls: ControlOverrides,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_compensation_steps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_balance_mode: Option<String>,
}

impl CameraProfile {
    /// Validate against known constraints. Returns Err with reason if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("profile name must not be empty".into());
        }
        if self.resolution.width == 0 || self.resolution.height == 0 {
            return Err("resolution must be non-zero".into());
        }
        if self.fps == 0 || self.fps > 120 {
            return Err(format!("fps {} out of range (1-120)", self.fps));
        }
        if self.camera_id.is_empty() {
            return Err("camera_id must not be empty".into());
        }
        Ok(())
    }
}

/// Atomic profile store: write-to-temp + rename pattern.
/// Survives crashes mid-write; invalid writes roll back cleanly.
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn profile_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }

    /// Save profile atomically. Validates before persisting.
    /// On any failure, existing file is unchanged.
    pub fn save(&self, profile: &CameraProfile) -> Result<(), String> {
        profile.validate()?;
        fs::create_dir_all(&self.dir).map_err(|e| format!("create dir: {e}"))?;

        let target = self.profile_path(&profile.name);
        let tmp = target.with_extension("json.tmp");

        let json = serde_json::to_string_pretty(profile).map_err(|e| format!("serialize: {e}"))?;
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("write tmp: {e}"))?;
        f.sync_all().map_err(|e| format!("fsync: {e}"))?;
        drop(f);

        fs::rename(&tmp, &target).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("rename: {e}")
        })?;
        Ok(())
    }

    /// Load profile by name. Returns Err if not found or corrupt.
    pub fn load(&self, name: &str) -> Result<CameraProfile, String> {
        let path = self.profile_path(name);
        let json =
            fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&json).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    /// List all profile names in the store.
    pub fn list(&self) -> Result<Vec<String>, String> {
        if !self.dir.exists() {
            return Ok(vec![]);
        }
        let mut names = vec![];
        for entry in fs::read_dir(&self.dir).map_err(|e| format!("readdir: {e}"))? {
            let entry = entry.map_err(|e| format!("entry: {e}"))?;
            if let Some(stem) = entry.path().file_stem() {
                if entry.path().extension().is_some_and(|ext| ext == "json") {
                    names.push(stem.to_string_lossy().into_owned());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Delete a profile. No-op if not found.
    pub fn delete(&self, name: &str) -> Result<(), String> {
        let path = self.profile_path(name);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_store(suffix: &str) -> (ProfileStore, PathBuf) {
        let dir = env::temp_dir().join(format!("camapro-profiles-{}-{suffix}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        (ProfileStore::new(&dir), dir)
    }

    fn valid_profile() -> CameraProfile {
        CameraProfile {
            name: "test".into(),
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
            fps: 30,
            camera_id: "0".into(),
            controls: ControlOverrides::default(),
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (store, dir) = temp_store("roundtrip");
        let p = valid_profile();
        store.save(&p).unwrap();
        let loaded = store.load("test").unwrap();
        assert_eq!(p, loaded);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_profile_rejected_before_write() {
        let (store, dir) = temp_store("invalid");
        let bad = CameraProfile {
            name: "".into(),
            ..valid_profile()
        };
        assert!(store.save(&bad).is_err());
        assert!(store.load("").is_err()); // nothing written
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrites_atomically() {
        let (store, dir) = temp_store("overwrite");
        let mut p = valid_profile();
        store.save(&p).unwrap();
        p.fps = 60;
        store.save(&p).unwrap();
        let loaded = store.load("test").unwrap();
        assert_eq!(loaded.fps, 60);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_returns_sorted_names() {
        let (store, dir) = temp_store("list");
        for name in ["beta", "alpha", "gamma"] {
            let mut p = valid_profile();
            p.name = name.into();
            store.save(&p).unwrap();
        }
        assert_eq!(store.list().unwrap(), vec!["alpha", "beta", "gamma"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_removes_profile() {
        let (store, dir) = temp_store("delete");
        store.save(&valid_profile()).unwrap();
        store.delete("test").unwrap();
        assert!(store.load("test").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
