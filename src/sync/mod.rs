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

pub mod protocol;
pub mod server;
pub mod secure;
pub mod family;
pub mod users;

pub use protocol::{SyncMessage, SyncCommand, SyncEvent};
pub use server::SyncServer;
pub use secure::{SecureSyncServer, SyncConfig, BindMode, TailscaleInfo};
pub use family::{FamilyConfig, FamilyDevice, TrustLevel};
pub use users::{UserManager, UserProfile, UserSession, UserLoginInfo, BootstrapResult};
