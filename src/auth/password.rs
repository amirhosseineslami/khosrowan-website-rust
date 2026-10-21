//! Password hashing and verification using Argon2id.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::{Algorithm, Argon2, Params, Version};

/// Hash a plaintext password with Argon2id.
pub fn hash_password(password: &str) -> crate::config::anyhow_lite::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = argon2();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| crate::config::anyhow_lite::Error::msg(format!("hash password: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored PHC hash string.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    argon2()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Argon2id with a memory-hard configuration sized for an interactive login.
fn argon2() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, None).expect("valid Argon2 parameters");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// A rough strength score from 0 to 4, used to nudge users toward better
/// passwords in the admin panel.
pub fn strength_score(password: &str) -> u8 {
    let mut score = 0u8;
    let len = password.chars().count();

    if len >= 8 {
        score += 1;
    }
    if len >= 12 {
        score += 1;
    }
    if len >= 16 {
        score += 1;
    }
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
    let classes = [has_lower, has_upper, has_digit, has_symbol]
        .iter()
        .filter(|x| **x)
        .count();
    if classes >= 3 {
        score += 1;
    }

    score.min(4)
}

/// Human readable label for a strength score.
pub fn strength_label(score: u8) -> &'static str {
    match score {
        0 | 1 => "Very weak",
        2 => "Weak",
        3 => "Good",
        _ => "Strong",
    }
}
