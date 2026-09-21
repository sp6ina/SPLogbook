fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "SPLogbook");
        res.set("FileDescription", "SPLogbook - Zaawansowany Dziennik Krotkofalarski");
        res.set("CompanyName", "Mariusz Wozniak (SP6INA)");
        res.set("LegalCopyright", "Copyright (C) 2026 Mariusz Wozniak (SP6INA)");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {}", e);
        }
    }
}
