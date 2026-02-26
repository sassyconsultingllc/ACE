//! Phone sync system - Secure, user-based sessions
//!
//! Connection model:
//! 1. Phone connects via Tailscale (encrypted mesh)
//! 2. Browser shows "Who's browsing?" with user avatars
//! 3. User picks their profile (optional PIN)
//! 4. User sees THEIR tabs, not everyone's
//! 5. Logout returns to user selection
//!
//! No ports visible to users. No IP addresses to remember.
//! Just "I'm Shane" -> here are Shane's tabs.
//!
//! Family model:
//! - First user becomes admin
//! - Admin can add/remove users
//! - Optional PIN per user
//! - NO surveillance - we don't track what users browse

pub mod family;
pub mod protocol;
pub mod secure;
pub mod server;
pub mod users;

pub use family::{FamilyConfig, TrustLevel};
pub use protocol::{SyncCommand, SyncEvent};
pub use secure::{SecureSyncServer, SyncConfig, TailscaleInfo};
pub use server::SyncServer;
pub use users::{UserLoginInfo, UserManager};

pub type SyncMessage = protocol::SyncMessage;
pub type BindMode = secure::BindMode;
pub type FamilyDevice = family::FamilyDevice;
pub type UserProfile = users::UserProfile;
pub type UserSession = users::UserSession;
pub type BootstrapResult = users::BootstrapResult;
