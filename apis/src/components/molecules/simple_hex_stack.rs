use crate::common::HexStack;
use crate::components::atoms::simple_hex::SimpleHex;
use crate::providers::config::TileOptions;
use leptos::prelude::*;

#[component]
pub fn SimpleHexStack(hex_stack: HexStack, tile_opts: TileOptions) -> impl IntoView {
    hex_stack
        .hexes
        .into_iter()
        .map(|hex| {
            view! { <SimpleHex hex=hex tile_opts=tile_opts.clone() /> }
        })
        .collect_view()
}
