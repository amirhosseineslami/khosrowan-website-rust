//! A self-contained RFC 6238 (TOTP) implementation.
//!
//! Implemented locally so the site has no extra dependency for a feature that
//! is only a few dozen lines of well-specified maths.

use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Number of digits in a generated code.
const DIGITS: u32 = 6;
/// Length of a time step in seconds.
pub const STEP_SECONDS: u64 = 30;

type HmacSha1 = Hmac<Sha1>;

/// Create a new random base32 secret (160 bits, RFC 4226 compatible).
pub fn generate_secret() -> String {
    let mut bytes = [0u8; 20];
    // `uuid::Uuid::new_v4` draws from the OS CSPRNG; we mix four of them to
    // collect 20 bytes of entropy.
    let mut i = 0;
    while i < bytes.len() {
        let u = uuid::Uuid::new_v4();
        let ub = u.as_bytes();
        let take = std::cmp::min(16, bytes.len() - i);
        bytes[i..i + take].copy_from_slice(&ub[..take]);
        i += take;
    }
    base32_encode(&bytes)
}

/// Compute the TOTP code for a given secret and unix timestamp.
pub fn code_at(secret: &str, timestamp: i64) -> Option<String> {
    let key = base32_decode(secret)?;
    let counter = (timestamp as u64) / STEP_SECONDS;
    let counter_bytes = counter.to_be_bytes();

    let mut mac = HmacSha1::new_from_slice(&key).ok()?;
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();

    let offset = (result[result.len() - 1] & 0x0f) as usize;
    let binary = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);

    let modulus = 10u32.pow(DIGITS);
    Some(format!(
        "{:0width$}",
        binary % modulus,
        width = DIGITS as usize
    ))
}

/// Verify a code against the secret, allowing for a small clock drift.
pub fn verify(secret: &str, code: &str, timestamp: i64) -> bool {
    let code = code.trim().replace(' ', "");
    if code.len() != DIGITS as usize || !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // Accept the current step and one step either side to tolerate clock drift.
    for window in [-1i64, 0, 1] {
        if let Some(expected) = code_at(secret, timestamp + window * STEP_SECONDS as i64) {
            if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
                return true;
            }
        }
    }
    false
}

/// Build the `otpauth://` URI used by authenticator apps.
pub fn provisioning_uri(secret: &str, account: &str, issuer: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        percent_encode(issuer),
        percent_encode(account),
        secret,
        percent_encode(issuer),
        DIGITS,
        STEP_SECONDS
    )
}

/// Generate a set of human-friendly recovery codes.
pub fn recovery_codes(count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            let raw = uuid::Uuid::new_v4().simple().to_string();
            format!("{}-{}", &raw[..5], &raw[5..10])
        })
        .collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// RFC 4648 base32 encoding without padding.
pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in data {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            let index = ((buffer >> (bits - 5)) & 0x1f) as usize;
            out.push(BASE32_ALPHABET[index] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(BASE32_ALPHABET[index] as char);
    }
    out
}

/// RFC 4648 base32 decoding, ignoring padding and whitespace.
pub fn base32_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for ch in input.chars() {
        if ch == '=' || ch == ' ' || ch == '-' {
            continue;
        }
        let upper = ch.to_ascii_uppercase() as u8;
        let value = BASE32_ALPHABET.iter().position(|&c| c == upper)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            out.push(((buffer >> (bits - 8)) & 0xff) as u8);
            bits -= 8;
        }
    }
    Some(out)
}

fn percent_encode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_roundtrip() {
        let data = b"hello world";
        let encoded = base32_encode(data);
        assert_eq!(base32_decode(&encoded).unwrap(), data);
    }

    #[test]
    fn rfc6238_reference_vector() {
        // The secret from RFC 6238 Appendix B, encoded in base32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        // Truncated to 20 bytes (the SHA-1 variant uses a 20-byte key).
        let secret = &secret[..32];
        assert_eq!(code_at(secret, 59).unwrap(), "287082");
        assert_eq!(code_at(secret, 1111111109).unwrap(), "081804");
        assert_eq!(code_at(secret, 1111111111).unwrap(), "050471");
    }

    #[test]
    fn verification_rejects_garbage() {
        let secret = &base32_encode(&[7u8; 20]);
        assert!(!verify(secret, "abcdef", 1_600_000_000));
        assert!(!verify(secret, "12345", 1_600_000_000));
    }
}
