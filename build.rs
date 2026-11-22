fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "OpenLauncher");
        res.set(
            "FileDescription",
            "OpenLauncher – Native Rust Game Launcher",
        );
        if let Err(err) = res.compile() {
            panic!("Windows-Ressourcen konnten nicht kompiliert werden: {err}");
        }
    }
}
