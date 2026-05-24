use super::model::{HivegroundRenderModel, RenderLayer};
use hive_lib::Position;
use leptos::prelude::*;
use std::collections::HashMap;

type LayersByPosition = HashMap<Position, Vec<RenderLayer>>;

pub fn layers_by_position(model: Memo<HivegroundRenderModel>) -> Memo<LayersByPosition> {
    Memo::new(move |_| {
        model.with(|model| {
            model
                .stacks
                .iter()
                .map(|stack| (stack.position, stack.layers.clone()))
                .collect()
        })
    })
}

pub fn layers_for_position(
    layers_by_position: Memo<LayersByPosition>,
    position: Position,
) -> Signal<Vec<RenderLayer>> {
    Signal::derive(move || {
        layers_by_position.with(|layers| layers.get(&position).cloned().unwrap_or_default())
    })
}

pub fn stack_positions(model: Memo<HivegroundRenderModel>) -> Signal<Vec<Position>> {
    Signal::derive(move || {
        model.with(|model| model.stacks.iter().map(|stack| stack.position).collect())
    })
}
