//! First-run setup wizard
//!
//! WHAT HAPPENS ON FIRST RUN:
//! ==============================================================================
//! 1. User enters their name
//! 2. Optionally sets a PIN
//! 3. Cryptographic keys generated:
//!    - Ed25519 identity key pair

//!    - Master secret for encryption
//!    - Device ID for sync
//! 4. Recovery key displayed (SAVE THIS!)
//! 5. Profile saved to disk (encrypted if PIN set)

use crate::data::{init_dirs, is_first_run, Config, UserData};
use crate::sync::{UserManager, TailscaleInfo};

/// Setup result
pub struct SetupResult {
    pub user_id: String,
    pub user_name: String,
    pub config: Config,
}

/// Run first-time setup
/// Returns None if user cancels
pub fn run_setup() -> Option<SetupResult> {
    // Initialize directories
    if let Err(e) = init_dirs() {
        eprintln!("Failed to create data directories: {}", e);
        return None;
    }
    
    // Check Tailscale status
    let tailscale = TailscaleInfo::detect();
    
    println!();
    println!("  Sassy Browser Setup");
    println!("  Secure - Private - Yours");
    println!();
    
    if tailscale.available {
        if let Some(ref hostname) = tailscale.hostname {
            println!("  [OK] Tailscale detected: {}", hostname);
        }
        if let Some(ref ip) = tailscale.ip {
            println!("  [OK] Tailscale IP: {}", ip);
        }
    } else {
        println!("  (web) Tailscale not detected");
        println!("    Phone sync requires Tailscale.");
        println!("    Get it at: https://tailscale.com/download");
    }
    
    use std::io::{self, Write};
    
    // Get user name
    println!("\n  Create Your Profile");
    println!();
    print!("  Your name: ");
    io::stdout().flush().ok();
    
    let mut name = String::new();
    if io::stdin().read_line(&mut name).is_err() {
        return None;
    }
    
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("  Setup cancelled.");
        return None;
    }
    
    // Optional PIN
    println!("\n  Set a PIN? (optional, protects your profile)");
    print!("  PIN (4-8 digits, or Enter to skip): ");
    io::stdout().flush().ok();
    
    let mut pin_input = String::new();
    if io::stdin().read_line(&mut pin_input).is_err() {
        return None;
    }
    
    let pin = pin_input.trim();
    let pin = if pin.is_empty() { 
        None 
    } else if pin.len() < 4 || pin.len() > 8 || !pin.chars().all(|c| c.is_ascii_digit()) {
        println!("  Invalid PIN format. Skipping PIN protection.");
        None
    } else {
        Some(pin)
    };
    
    // Create user with cryptographic identity
    let mut user_data = UserData::default();
    
    println!("\n  Generating cryptographic identity...");
    
    let result = match user_data.users.bootstrap(name.clone(), pin) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  Failed to create user: {}", e);
            return None;
        }
    };
    
    let user_id = result.user_id.clone();
    let recovery_key = result.recovery_key;
    
    // Save user data
    if let Err(e) = user_data.save() {
        eprintln!("  Failed to save user data: {}", e);
        return None;
    }
    
    // Create config
    let config = Config::default();
    if let Err(e) = config.save() {
        eprintln!("  Failed to save config: {}", e);
        return None;
    }
    
    // CRITICAL: Display recovery key
    println!();
    println!();
    println!("  RECOVERY KEY - SAVE THIS!");
    println!();
    println!();
    println!("  {}", format_recovery_key(&recovery_key));
    println!();
    println!("  This key can restore your profile if you forget your");
    println!("  PIN. Write it down and store it somewhere safe.");
    println!();
    println!("  WITHOUT THIS KEY, A FORGOTTEN PIN = LOST DATA");
    println!();
    println!();
    
    // Security summary
    println!("  Profile Created: {}", name);
    println!();
    if pin.is_some() {
        println!("  [OK] PIN protection enabled");
    } else {
        println!("  (web) No PIN (anyone with computer access can use)");
    }
    println!("  [OK] Ed25519 identity key generated");
    println!("  [OK] Data encryption key derived");
    println!("  [OK] You are the admin");
    
    if tailscale.available {
        println!();
        println!("  Phone Sync Ready!");
        if let Some(hostname) = tailscale.hostname {
            println!("  Connect from phone using: {}", hostname);
        }
    }
    
    println!("\n  Press Enter to start browsing...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
    
    Some(SetupResult {
        user_id,
        user_name: name,
        config,
    })
}

/// Format recovery key for display (add spaces for readability)
fn format_recovery_key(key: &str) -> String {
    // Pad/truncate to fit box
    let formatted: String = key.chars()
        .take(43)  // Max width that fits in box
        .collect();
    format!("{:^43}", formatted)
}

/// Check if setup is needed and run if so
pub fn ensure_setup() -> Option<SetupResult> {
    if is_first_run() {
        run_setup()
    } else {
        // Load existing config
        let config = Config::load();
        let user_data = UserData::load();
        
        // Get first admin user
        let admin = user_data.users.list_users()
            .into_iter()
            .find(|u| u.is_admin);
        
        if let Some(user) = admin {
            Some(SetupResult {
                user_id: user.id.clone(),
                user_name: user.name.clone(),
                config,
            })
        } else {
            // No users - run setup
            run_setup()
        }
    }
}
