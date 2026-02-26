// Build script for Windows resources (icon, manifest)

fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        res.set("ProductName", "Sassy Browser");
        res.set("FileDescription", "Pure Rust Web Browser");
        res.set("LegalCopyright", "Sassy Consulting LLC");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
