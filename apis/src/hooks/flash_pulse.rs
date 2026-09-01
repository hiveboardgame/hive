use crate::providers::flash::FlashSignal;
use leptos::{leptos_dom::helpers::set_timeout, prelude::*};
use shared_types::FlashStyle;
use std::time::Duration;

const FLASH_DURATION: Duration = Duration::from_millis(1250);

pub fn use_flash_pulse(style: FlashStyle) -> Signal<bool> {
    let flash = expect_context::<FlashSignal>();
    let active = RwSignal::new(false);
    Effect::new(move |previous: Option<Option<u32>>| {
        let pulse = flash.pulse.get();
        let id = pulse.as_ref().map(|pulse| pulse.id);
        let fresh = previous.is_some_and(|seen| seen != id);
        if fresh && pulse.is_some_and(|pulse| pulse.styles.contains(&style)) {
            active.set(true);
            set_timeout(move || active.set(false), FLASH_DURATION);
        }
        id
    });
    active.into()
}
