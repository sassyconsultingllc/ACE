use std::process::Command;

fn main() {
    // When invoked via `cargo run --bin sassy-ui`, Cargo sets this env var
    // pointing to the built `sassy-browser` binary. Prefer that if present.
    let sassy_exe = std::env::var("CARGO_BIN_EXE_sassy-browser").unwrap_or_else(|_| {
        // Fallback: try to execute `sassy-browser` from PATH
        "sassy-browser".to_string()
    });

    // Forward any provided args to the main browser binary.
    let args: Vec<String> = std::env::args().skip(1).collect();

    let status = Command::new(&sassy_exe)
        .args(&args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to spawn '{}': {}", sassy_exe, e);
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(0));
}
