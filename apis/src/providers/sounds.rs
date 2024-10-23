use super::Config;
use leptos::*;
use rand::Rng;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::ArrayBuffer, AudioBuffer, AudioContext, Response};

#[derive(Clone, Debug, PartialEq)]
pub enum SoundType {
    Turn,
    NewGame,
    LowTime,
}

#[derive(Clone)]
pub struct SoundsSignal {
    pub context: RwSignal<Option<AudioContext>>,
    pub turn: RwSignal<Option<AudioBuffer>>,
    pub low: RwSignal<Option<AudioBuffer>>,
    pub new: RwSignal<Option<AudioBuffer>>,
}

impl SoundsSignal {
    pub fn play_sound(&self, kind: SoundType) {
        let config = expect_context::<Config>().0;
        if !config().prefers_sound {
            return;
        };
        if let Some(context) = self.context.get() {
            let (signal, offset, duration) = match kind {
                SoundType::Turn => (self.turn, rand::thread_rng().gen_range(0..20) as f64, 1.0),
                SoundType::NewGame => (self.new, 0.0, 3.0),
                SoundType::LowTime => (self.low, 0.0, 2.0),
            };
            if let Some(buffer) = signal.get_untracked() {
                let source = context.create_buffer_source().unwrap();
                source.set_buffer(Some(&buffer));
                source.set_loop(false);
                source
                    .start_with_when_and_grain_offset_and_grain_duration(0.0, offset, duration)
                    .unwrap();
                source
                    .connect_with_audio_node(&context.destination())
                    .unwrap();
            }
        }
    }
}

pub async fn load_audio_buffer(context: &AudioContext, url: &str) -> Result<AudioBuffer, JsValue> {
    let response_value = JsFuture::from(window().fetch_with_str(url)).await?;
    let response: Response = response_value.dyn_into()?;
    let array_buffer_value = JsFuture::from(response.array_buffer()?).await?;
    let array_buffer: ArrayBuffer = array_buffer_value.dyn_into()?;
    let audio_buffer_value = JsFuture::from(context.decode_audio_data(&array_buffer)?).await?;
    let audio_buffer: AudioBuffer = audio_buffer_value.dyn_into()?;
    Ok(audio_buffer)
}

pub fn provide_sounds() {
    let context = RwSignal::new(None);
    let turn = RwSignal::new(None);
    let low = RwSignal::new(None);
    let new = RwSignal::new(None);
    provide_context(SoundsSignal {
        context,
        turn,
        low,
        new,
    });
}
