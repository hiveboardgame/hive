pub fn handle_oauth(link: String) {
    // INFO: If you want it to open in a new window use:
    //.open_with_url_and_target(&link, "_blank")
    web_sys::window()
        .unwrap()
        .location()
        .set_href(&link)
        .expect("Failed to open window");
}
