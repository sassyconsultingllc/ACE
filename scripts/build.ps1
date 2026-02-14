Param()
Set-StrictMode -Version Latest
Write-Host "Ensure Rust (rustup) + stable toolchain are installed: https://rustup.rs"
rustup default stable 2>$null
Write-Host "cargo fmt --check"
cargo fmt --all -- --check
Write-Host "cargo check"
cargo check --workspace --all-targets
Write-Host "cargo build --release"
cargo build --workspace --release
Write-Host "Build finished: target\release\"
