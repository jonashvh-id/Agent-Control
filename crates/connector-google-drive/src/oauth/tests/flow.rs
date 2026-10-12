use super::*;
#[tokio::test]
async fn oauth_state_cookie_session_and_one_time_binding_are_enforced() {
    let f = fixture().await;
    let state = f.start().await;
    assert!(
        f.manager
            .finish("wrong-cookie".into(), Fixture::callback(&state))
            .await
            .is_err()
    );
    assert_eq!(f.native.tokens.load(Ordering::SeqCst), 0);
    f.manager
        .finish("fixture-cookie-hash".into(), Fixture::callback(&state))
        .await
        .unwrap();
    assert!(
        f.manager
            .finish("fixture-cookie-hash".into(), Fixture::callback(&state))
            .await
            .is_err()
    );
    assert_eq!(f.native.tokens.load(Ordering::SeqCst), 1);
    let status = f.manager.status().await.unwrap().to_string();
    for secret in ["access-fixture", "refresh-fixture", "client-secret-fixture"] {
        assert!(!status.contains(secret));
    }
    let cipher: String = sqlx::query_scalar("SELECT ciphertext FROM connection_secrets")
        .fetch_one(&f.app.db.pool)
        .await
        .unwrap();
    assert!(!cipher.contains("refresh-fixture"));
    assert!(
        f.app
            .db
            .keys
            .open_private("connection:other", &cipher)
            .is_err()
    );
    let audit = control_core::audit::list(&f.app.db.pool)
        .await
        .unwrap()
        .to_string();
    for raw in [
        "access-fixture",
        "refresh-fixture",
        "client-secret-fixture",
        &state,
    ] {
        assert!(!audit.contains(raw));
    }
}
#[tokio::test]
async fn logout_expiry_disabled_admin_and_issuer_stop_before_exchange() {
    for scenario in 0..4 {
        let f = fixture().await;
        let state = f.start().await;
        let mut callback = Fixture::callback(&state);
        match scenario {
            0 => {
                sqlx::query("DELETE FROM sessions")
                    .execute(&f.app.db.pool)
                    .await
                    .unwrap();
            }
            1 => {
                sqlx::query("UPDATE oauth_attempts SET expires_at=0")
                    .execute(&f.app.db.pool)
                    .await
                    .unwrap();
            }
            2 => {
                sqlx::query("UPDATE humans SET enabled=0")
                    .execute(&f.app.db.pool)
                    .await
                    .unwrap();
            }
            _ => callback.issuer = Some("https://attacker.invalid".into()),
        };
        assert!(
            f.manager
                .finish("fixture-cookie-hash".into(), callback)
                .await
                .is_err()
        );
        assert_eq!(f.native.tokens.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn broader_scopes_and_non_bearer_tokens_are_not_stored() {
    for mode in [1, 2] {
        let f = fixture().await;
        f.native.token_mode.store(mode, Ordering::SeqCst);
        let state = f.start().await;
        assert!(
            f.manager
                .finish("fixture-cookie-hash".into(), Fixture::callback(&state))
                .await
                .is_err()
        );
        assert_eq!(f.manager.status().await.unwrap()["state"], "disconnected");
        assert_eq!(f.native.calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn refresh_is_serialized_and_invalid_grant_requires_reconnect() {
    let f = fixture().await;
    f.connect().await;
    let (_, data, generation, _) = f.manager.store.read("google-drive").await.unwrap();
    let mut data = data.unwrap();
    data["expires_at"] = json!(0);
    f.manager
        .store
        .refresh("google-drive", generation, &data)
        .await
        .unwrap();
    let (a, b) = tokio::join!(f.manager.token(), f.manager.token());
    assert_eq!(&*a.unwrap(), "access-fixture");
    assert_eq!(&*b.unwrap(), "access-fixture");
    assert_eq!(f.native.tokens.load(Ordering::SeqCst), 2);
    let (_, data, generation, _) = f.manager.store.read("google-drive").await.unwrap();
    let mut data = data.unwrap();
    data["expires_at"] = json!(0);
    f.manager
        .store
        .refresh("google-drive", generation, &data)
        .await
        .unwrap();
    f.native.token_mode.store(3, Ordering::SeqCst);
    assert_eq!(
        f.manager.token().await.err().unwrap().code,
        "google_reauthentication_required"
    );
    assert_eq!(
        f.manager.status().await.unwrap()["state"],
        "reauth_required"
    );
    assert_eq!(f.manager.store.read("google-drive").await.unwrap().1, None);
}
#[tokio::test]
async fn revoked_session_during_exchange_cannot_store_a_connection() {
    let f = fixture().await;
    let state = f.start().await;
    f.native.token_mode.store(4, Ordering::SeqCst);
    let m = f.manager.clone();
    let task = tokio::spawn(async move {
        m.finish("fixture-cookie-hash".into(), Fixture::callback(&state))
            .await
    });
    f.native.entered.notified().await;
    sqlx::query("DELETE FROM sessions")
        .execute(&f.app.db.pool)
        .await
        .unwrap();
    f.native.released.notify_one();
    assert!(task.await.unwrap().is_err());
    assert_eq!(f.manager.status().await.unwrap()["state"], "disconnected");
}
#[tokio::test]
async fn disconnect_invalidates_pending_callbacks_and_old_verification() {
    let f = fixture().await;
    f.connect().await;
    let (generation, revision) = f.manager.store.revision("google-drive").await.unwrap();
    let state = f.start().await;
    assert_eq!(
        f.manager.disconnect().await.unwrap()["native_revocation_confirmed"],
        true
    );
    assert!(
        f.manager
            .finish("fixture-cookie-hash".into(), Fixture::callback(&state))
            .await
            .is_err()
    );
    assert!(
        f.manager
            .store
            .verification_checked(
                "google-drive",
                &json!({"verified":true}),
                generation,
                revision
            )
            .await
            .is_err()
    );
    assert_eq!(f.manager.status().await.unwrap()["state"], "disconnected");
    assert_eq!(f.native.tokens.load(Ordering::SeqCst), 1);
}
