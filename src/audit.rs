//! Audit logging for administrative actions.

use crate::state::AppState;

/// Record an action in the audit log. Failures are logged but never fatal.
pub async fn audit(
    state: &AppState,
    user_id: Option<i64>,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    details: Option<&str>,
) {
    let username = match user_id {
        Some(id) => sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten(),
        None => None,
    };

    let result = sqlx::query(
        "INSERT INTO audit_log (user_id, username, action, entity_type, entity_id, details)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(&username)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .execute(&state.db)
    .await;

    if let Err(err) = result {
        tracing::warn!(error = %err, action, "failed to write audit log");
    }
}
