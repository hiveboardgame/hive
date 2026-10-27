use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct SiteFooter {
    // A count, not a flag: during navigation the next page may register before the previous one cleans up.
    pages: RwSignal<usize>,
    container_class: RwSignal<&'static str>,
}

impl SiteFooter {
    pub fn provide() -> Self {
        let site_footer = Self {
            pages: RwSignal::new(0),
            container_class: RwSignal::new(""),
        };
        provide_context(site_footer);
        site_footer
    }

    pub fn is_shown(self) -> bool {
        self.pages.with(|count| *count > 0)
    }

    pub fn container_class(self) -> &'static str {
        self.container_class.get()
    }
}

pub fn use_site_footer(container_class: &'static str) {
    let Some(site_footer) = use_context::<SiteFooter>() else {
        return;
    };
    site_footer.pages.update(|count| *count += 1);
    site_footer.container_class.set(container_class);
    on_cleanup(move || {
        site_footer
            .pages
            .try_update(|count| *count = count.saturating_sub(1));
    });
}
