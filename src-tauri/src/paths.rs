use ditovr_core::config;
use std::{io, path::Path};

pub const CONFIG_DIRECTORY: &str = "DiToVR";
pub const PREVIOUS_CONFIG_DIRECTORY: &str = "discord-to-vr";

/// Prefer the previous per-user settings over an older portable copy.
/// Migration errors must not fall back to an unrelated account configuration.
pub fn migrate_config(destination: &Path, sources: &[&Path]) -> io::Result<bool> {
    if destination.try_exists()? {
        return Ok(false);
    }
    for source in sources {
        if source.try_exists()? {
            return config::migrate_config_at(source, destination);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn user_settings_take_priority_and_new_settings_are_never_overwritten() {
        let dir = std::env::temp_dir().join(format!(
            "discord-vr-paths-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let previous = dir.join("previous.json");
        let older = dir.join("older.json");
        let portable = dir.join("portable.json");
        let destination = dir.join("destination.json");
        let document = |id: &str| {
            serde_json::json!({
                "CLIENT_ID":id, "CLIENT_SECRET":"synthetic-secret", "ACCESS_TOKEN":"synthetic-access",
                "PRIVACY_MODE":true, "CUSTOM_SETTING":"keep"
            })
        };
        let original = serde_json::to_vec(&document("123456789")).unwrap();
        fs::write(&previous, &original).unwrap();
        fs::write(&older, serde_json::to_vec(&document("222222222")).unwrap()).unwrap();
        fs::write(
            &portable,
            serde_json::to_vec(&document("987654321")).unwrap(),
        )
        .unwrap();
        assert!(migrate_config(&destination, &[&previous, &older, &portable]).unwrap());
        let migrated = config::load_from_path(&destination, false).unwrap();
        assert_eq!(migrated.client_id, "123456789");
        assert_eq!(migrated.access_token, "synthetic-access");
        assert!(migrated.privacy_mode);
        assert_eq!(fs::read(&previous).unwrap(), original);
        let saved = fs::read(&destination).unwrap();
        let raw: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        assert_eq!(raw["CUSTOM_SETTING"], "keep");
        assert!(raw["CLIENT_SECRET"]
            .as_str()
            .unwrap()
            .starts_with("dpapi:v1:"));
        fs::write(&previous, b"{invalid").unwrap();
        assert!(!migrate_config(&destination, &[&previous, &portable]).unwrap());
        assert_eq!(fs::read(&destination).unwrap(), saved);
        fs::remove_file(&destination).unwrap();
        assert!(migrate_config(&destination, &[&previous, &older, &portable]).is_err());
        assert!(!destination.exists());
        fs::remove_file(&previous).unwrap();
        assert!(migrate_config(&destination, &[&previous, &older, &portable]).unwrap());
        assert_eq!(
            config::load_from_path(&destination, false)
                .unwrap()
                .client_id,
            "222222222"
        );
        fs::remove_file(&destination).unwrap();
        fs::remove_file(&older).unwrap();
        assert!(migrate_config(&destination, &[&previous, &portable]).unwrap());
        assert_eq!(
            config::load_from_path(&destination, false)
                .unwrap()
                .client_id,
            "987654321"
        );
        for path in [&destination, &portable] {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
}
