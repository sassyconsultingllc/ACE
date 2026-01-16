@echo off
cd /d "%~dp0"
echo Building and running Sassy Browser...
cargo run --release -- %*
