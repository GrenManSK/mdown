fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources\\icon.ico");
        match res.compile() {
            Ok(_) => (),
            Err(_err) => (),
        };
    }
}
