fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=APP_BUILD");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Ordis Nexus");
        res.set("FileDescription", "Ordis Nexus");
        res.compile().expect("failed to embed Windows resources");
    }
}
