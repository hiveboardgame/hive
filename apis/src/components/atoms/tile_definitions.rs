use crate::{
    common::{inline_tile_sprite, namespace_svg, THREE_D_SHADOW_PATH, THREE_D_SHADOW_PREFIX},
    providers::Config,
};
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(not(feature = "ssr"))]
use wasm_bindgen_futures::JsFuture;
#[cfg(not(feature = "ssr"))]
use web_sys::Response;

/// WebKit loses gradients and filters through external SVG use references.
/// Share one set of local definitions across boards, reserves, history and thumbnails.
#[component]
pub fn TileDefinitions() -> impl IntoView {
    let config = expect_context::<Config>().0;
    let sprite = Memo::new(move |_| config.with(|config| inline_tile_sprite(&config.tile.design)));
    let definitions = LocalResource::new(move || {
        let sprite = sprite.get();
        async move {
            let Some((path, prefix)) = sprite else {
                return String::new();
            };
            let result = async {
                let tiles = fetch_svg(path).await?;
                let shadow = fetch_svg(THREE_D_SHADOW_PATH).await?;
                Ok::<_, String>(format!(
                    "{}{}",
                    namespace_svg(&tiles, prefix),
                    namespace_svg(&shadow, THREE_D_SHADOW_PREFIX)
                ))
            }
            .await;
            match result {
                Ok(svg) => svg,
                Err(error) => {
                    log::warn!("Could not load tile definitions: {error}");
                    String::new()
                }
            }
        }
    });

    // Keep this outside display:none ancestors: WebKit also drops their paint servers.
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="0"
            height="0"
            aria-hidden="true"
            class="absolute pointer-events-none"
            inner_html=move || definitions.get().unwrap_or_default()
        ></svg>
    }
}

#[cfg(not(feature = "ssr"))]
async fn fetch_svg(path: &str) -> Result<String, String> {
    let response: Response = JsFuture::from(window().fetch_with_str(path))
        .await
        .and_then(|response| response.dyn_into())
        .map_err(|error: JsValue| format!("{path}: {error:?}"))?;
    if !response.ok() {
        return Err(format!("{path}: HTTP {}", response.status()));
    }
    JsFuture::from(
        response
            .text()
            .map_err(|error| format!("{path}: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("{path}: {error:?}"))?
    .as_string()
    .ok_or_else(|| format!("{path}: response was not text"))
}

#[cfg(feature = "ssr")]
async fn fetch_svg(_path: &str) -> Result<String, String> {
    // LocalResource starts only in the browser; no asset markup is added to SSR or WASM.
    Ok(String::new())
}
