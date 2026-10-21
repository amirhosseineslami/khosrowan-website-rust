//! Authentication primitives: passwords, sessions and TOTP.

pub mod password;
pub mod session;
pub mod totp;

pub use session::AuthContext;

/// The maximum number of consecutive failed logins before an account is
/// temporarily locked.
pub const MAX_FAILED_LOGINS: i64 = 8;
/// How long an account stays locked after too many failures.
pub const LOCKOUT_MINUTES: i64 = 15;
