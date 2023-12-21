use leptos::*;
use leptos_router::*;

#[derive(Params, PartialEq, Eq, Debug)]
pub struct PlayParams {
    pub nanoid: String,
}
