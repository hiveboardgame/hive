// use leptos_router::{use_navigate, NavigateOptions};

pub fn handle_oauth(link: String) {
    // let navigate = use_navigate();
    // let options = NavigateOptions {
    //     resolve: false,
    //     ..Default::default()
    // };
    // navigate(&link, options);
    web_sys::window()
        .unwrap()
        .open_with_url_and_target(&link, "_blank")
        .expect("Failed to open window");
}
