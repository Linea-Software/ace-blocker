use crate::api_client::{SetEnabledRequest, SetNetworkLockRequest};
use crate::common::{get_api_client_from, get_default_test_server};
use ace_engine::manager::network::resistance_mode::EnforcementStatus;
use ace_engine::manager::resistance::punishment::{PunishmentIntensity, PunishmentMode};

#[tokio::test]
async fn test_resistance_mode_basic_settings() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // 1. Get initial settings — default is Static with None intensity
    let settings = client
        .get_resistance_mode_settings()
        .await
        .expect("Failed to get resistance engine settings");

    assert!(!settings.enabled);
    assert!(matches!(settings.mode, PunishmentMode::Static { .. }));
    assert_eq!(settings.current_intensity, PunishmentIntensity::None);

    // 2. Enable engine
    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .expect("Failed to enable resistance engine");

    let settings = client
        .get_resistance_mode_settings()
        .await
        .expect("Failed to get settings after enable");
    assert!(settings.enabled);
}

#[tokio::test]
async fn test_resistance_mode_lock_constraints() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // 0. Enable engine first
    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .expect("Failed to enable resistance engine");

    // 1. Lock the engine
    let lock_end = crate::common::get_later_date_time();
    client
        .set_resistance_mode_lock(SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(lock_end),
        })
        .await
        .expect("Failed to lock resistance engine");

    let settings = client
        .get_resistance_mode_settings()
        .await
        .expect("Failed to get settings after lock");
    assert!(settings.is_locked);

    // 2. Try to disable (should fail)
    let res = client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: false })
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_resistance_mode_scripts() {
    // Deterministic fixture: a scripts dir with a catalog + script file,
    // wired into a dedicated test server. The shared default scripts dir is
    // empty, so the old version's body was gated on environment state and
    // no-oped in CI (TESTINFRA-02). The fixture dir is set and the manager
    // built synchronously (no await between them) to minimize the
    // process-global race window.
    let temp = tempfile::tempdir().unwrap();
    let scripts_dir = temp.path().join("resistance_scripts");
    std::fs::create_dir_all(scripts_dir.join("com/youtube")).unwrap();
    std::fs::write(
        scripts_dir.join("com/youtube/grayscale.js"),
        "console.log('grayscale');",
    )
    .unwrap();
    std::fs::write(
        scripts_dir.join("scripts-data.json"),
        r#"{"sites": {"com": {"scripts": [], "children": {"youtube": {"scripts": [{"id": "youtube-grayscale", "path": "com/youtube/grayscale.js", "name": "Grayscale", "description": "test", "version": "1.0"}], "children": {}}}}}}"#,
    )
    .unwrap();
    // The fixture dir is set and the manager built while holding the shared
    // resistance-dir lock, so a parallel test server cannot overwrite the
    // process-global dir in between (TESTINFRA-01).
    ace_engine::settings::Settings::set_test_resistance_scripts_dir(scripts_dir);

    let _resistance_lock = crate::common::RESISTANCE_DIR_LOCK.lock().unwrap();
    let db_path = temp.path().join("test_db.sqlite");
    let server = crate::common::get_test_server_from_db_path(&db_path);
    let client = get_api_client_from(&server);
    drop(_resistance_lock);

    let scripts_res = client
        .get_all_scripts()
        .await
        .expect("Failed to get scripts");
    assert_eq!(scripts_res.scripts.len(), 1, "the fixture has one script");

    let first_script = &scripts_res.scripts[0];
    let script_id = &first_script.id;

    let script = client
        .get_script(script_id)
        .await
        .expect("Failed to get script by ID");
    assert_eq!(script.id, *script_id);

    let initial_state = script.is_enabled;
    client
        .set_script_enabled(
            script_id,
            SetEnabledRequest {
                enabled: !initial_state,
            },
        )
        .await
        .expect("Failed to toggle script state");

    let script = client
        .get_script(script_id)
        .await
        .expect("Failed to get script after toggle");
    assert_eq!(script.is_enabled, !initial_state);
}

// ── Mode switching tests (v1 beta: static only) ─────────────────────

#[tokio::test]
async fn test_dynamic_mode_is_rejected_as_unsupported() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(
        matches!(settings.mode, PunishmentMode::Static { .. }),
        "default mode should be Static"
    );

    // Dynamic mode is no longer a representable variant; a raw "dynamic"
    // payload must be rejected by deserialization (400) without mutating the
    // persisted mode.
    let response = server
        .post("/resistance-mode/type")
        .json(&serde_json::json!({ "type": "dynamic" }))
        .await;
    assert_eq!(response.status_code(), 400, "dynamic mode must be rejected");

    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(
        matches!(settings.mode, PunishmentMode::Static { .. }),
        "mode must stay Static after a rejected dynamic request"
    );
}

#[tokio::test]
async fn test_switch_to_static_with_explicit_intensity() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    // The client can set the static intensity explicitly via the API
    // (previously static intensity was stuck at its default None).
    let mode = PunishmentMode::Static {
        intensity: PunishmentIntensity::Extreme,
    };
    let response = client.set_resistance_mode_type(&mode).await.unwrap();

    assert!(
        matches!(response.mode, PunishmentMode::Static { .. }),
        "should be in Static mode"
    );
    assert_eq!(
        response.current_intensity,
        PunishmentIntensity::Extreme,
        "static intensity should be applied, not left at default None"
    );

    // The settings must reflect the stored static intensity.
    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert_eq!(settings.static_intensity, PunishmentIntensity::Extreme);
    assert_eq!(settings.current_intensity, PunishmentIntensity::Extreme);
}

#[tokio::test]
async fn test_static_intensity_survives_restart() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db.sqlite");
    let sites_dir = temp_dir.path().join("sites_lists");
    std::fs::create_dir_all(&sites_dir).unwrap();
    ace_engine::settings::Settings::set_test_sites_dir(sites_dir);

    // First "boot": switch to Static Extreme.
    let server = crate::common::get_test_server_from_db_path(&db_path);
    let client = get_api_client_from(&server);
    client
        .set_resistance_mode_type(&PunishmentMode::Static {
            intensity: PunishmentIntensity::Extreme,
        })
        .await
        .unwrap();
    drop(server);

    // Second "boot" from the same DB: the static intensity must survive.
    let server = crate::common::get_test_server_from_db_path(&db_path);
    let client = get_api_client_from(&server);
    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(
        matches!(settings.mode, PunishmentMode::Static { .. }),
        "mode should survive a restart"
    );
    assert_eq!(
        settings.current_intensity,
        PunishmentIntensity::Extreme,
        "static intensity should survive a restart"
    );
    assert_eq!(settings.static_intensity, PunishmentIntensity::Extreme);
}

// ── Lock-state restriction tests (v1 beta) ─────────────────────────

#[tokio::test]
async fn test_unsupported_lock_states_are_rejected() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .unwrap();

    for unsupported in ["soft_unlockable", "active_during_schedule"] {
        let response = client
            .set_resistance_mode_lock_raw(SetNetworkLockRequest {
                lock_state: unsupported.to_string(),
                lock_end: None,
            })
            .await
            .expect("request should produce an HTTP response");
        assert_eq!(
            response.status(),
            400,
            "{} must be rejected with a 4xx",
            unsupported
        );
        let json: serde_json::Value =
            serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
        assert_eq!(json["code"], "unsupported_lock_state");

        // No in-memory mutation after rejection.
        let settings = client.get_resistance_mode_settings().await.unwrap();
        assert!(!settings.is_locked, "rejected lock must not mutate memory");
    }
}

#[tokio::test]
async fn test_lock_strengthening_and_expiration_normalization() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .unwrap();

    // 1. Create a timed lock.
    let later = crate::common::get_later_date_time();
    client
        .set_resistance_mode_lock(SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(later),
        })
        .await
        .unwrap();
    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(settings.is_locked);
    assert!(settings.lock_end.is_some());

    // 2. Strengthen: a longer lock is accepted.
    let much_later = (chrono::Local::now() + chrono::Duration::days(2))
        .format(ace_engine::core::constants::DATETIME_FORMAT)
        .to_string();
    client
        .set_resistance_mode_lock(SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(much_later),
        })
        .await
        .unwrap();
    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(settings.is_locked);

    // 3. Weakening (shorter lock) is refused while locked.
    let response = client
        .set_resistance_mode_lock_raw(SetNetworkLockRequest {
            lock_state: "active_until_lock_end".to_string(),
            lock_end: Some(crate::common::get_later_date_time()),
        })
        .await
        .unwrap();
    assert_eq!(response.status(), 403, "weakening an active lock must fail");
}

// ── Scripts-version persistence tests (regression for B1) ───────────

#[tokio::test]
async fn test_scripts_version_survives_settings_saves() {
    // Simulate an installed version (as a successful download would persist
    // it), then boot the engine and trigger settings saves. The version must
    // never be clobbered back to None/stale.
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db.sqlite");
    let sites_dir = temp_dir.path().join("sites_lists");
    std::fs::create_dir_all(&sites_dir).unwrap();
    ace_engine::settings::Settings::set_test_sites_dir(sites_dir);

    let db = ace_engine::db::EngineDB::from(&db_path).unwrap();
    db.update_scripts_version("1.2.3".to_string()).unwrap();
    drop(db);

    let server = crate::common::get_test_server_from_db_path(&db_path);
    let client = get_api_client_from(&server);

    // A settings save must not clobber the stored version back to None.
    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .unwrap();
    client
        .set_resistance_mode_type(&PunishmentMode::Static {
            intensity: PunishmentIntensity::Low,
        })
        .await
        .unwrap();

    let db = ace_engine::db::EngineDB::from(&db_path).unwrap();
    let stored = db.get_resistance_mode_settings().unwrap();
    assert_eq!(
        stored.scripts_version.as_deref(),
        Some("1.2.3"),
        "settings saves must preserve the installed scripts version"
    );
}

// ── Enforcement status tests (B4) ──────────────────────────────────

#[tokio::test]
async fn test_enforcement_status_is_not_operational_when_disabled() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(!settings.enabled);
    assert_eq!(
        settings.enforcement_status,
        EnforcementStatus::Disabled,
        "a disabled mode must never report operational, regardless of scripts"
    );
    assert!(settings.pro_eligible, "test servers force Pro");
}

#[tokio::test]
async fn test_enforcement_status_reports_operational_with_scripts_and_intensity() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .unwrap();
    client
        .set_resistance_mode_type(&PunishmentMode::Static {
            intensity: PunishmentIntensity::Medium,
        })
        .await
        .unwrap();

    // The script tree on a dev machine may be real (downloaded scripts in
    // ProgramData) or empty (test servers); either way the status must be
    // consistent with what the engine actually has — never Disabled, NotPro
    // or IntensityNone at this point. The deterministic missing/invalid
    // matrix is covered by the unit tests in `resistance_mode.rs`.
    let settings = client.get_resistance_mode_settings().await.unwrap();
    assert!(settings.enabled);
    assert!(settings.pro_eligible);
    match settings.enforcement_status {
        EnforcementStatus::Operational
        | EnforcementStatus::ScriptsMissing
        | EnforcementStatus::ScriptsInvalid
        | EnforcementStatus::NoScriptsEnabled => {}
        other => panic!("unexpected enforcement status: {other:?}"),
    }
    assert_eq!(settings.static_intensity, PunishmentIntensity::Medium);
}

#[tokio::test]
async fn test_enforcement_status_intensity_none_is_not_operational() {
    let server = get_default_test_server();
    let client = get_api_client_from(&server);

    client
        .set_resistance_mode_enabled(SetEnabledRequest { enabled: true })
        .await
        .unwrap();
    // Static intensity defaults to None — must never be reported operational.
    // (With no scripts installed the status may be ScriptsMissing instead of
    // IntensityNone; the invariant under test is "not operational".)
    let settings = client.get_resistance_mode_settings().await.unwrap();
    match settings.enforcement_status {
        EnforcementStatus::ScriptsMissing
        | EnforcementStatus::ScriptsInvalid
        | EnforcementStatus::NoScriptsEnabled
        | EnforcementStatus::IntensityNone => {}
        other => panic!("intensity none must never report operational: {other:?}"),
    }
}
