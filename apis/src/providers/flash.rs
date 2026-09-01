use leptos::prelude::*;
use shared_types::FlashStyle;

#[derive(Clone, Debug, PartialEq)]
pub struct FlashPulse {
    pub id: u32,
    pub styles: Vec<FlashStyle>,
}

#[derive(Clone, Copy)]
pub struct FlashSignal {
    pub pulse: RwSignal<Option<FlashPulse>>,
}

impl FlashSignal {
    pub fn fire(&self, styles: Vec<FlashStyle>) {
        self.pulse.update(|pulse| {
            let id = pulse.as_ref().map_or(0, |last| last.id.wrapping_add(1));
            *pulse = Some(FlashPulse { id, styles });
        });
    }
}

pub fn provide_flash() {
    provide_context(FlashSignal {
        pulse: RwSignal::new(None),
    });
}
