use markdown::{Constructs, Options, ParseOptions};

pub fn markdown_to_html(markdown: &str) -> Option<String> {
    let options = &Options {
        parse: ParseOptions {
            constructs: Constructs {
                label_start_image: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    markdown::to_html_with_options(markdown, options).ok()
}
