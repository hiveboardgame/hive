use crate::functions::config::sound::ToggleSound;
use leptos::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use rand::Rng;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys::ArrayBuffer, AudioBuffer, AudioBufferSourceNode, AudioContext, Response};

#[cfg(not(feature = "ssr"))]
fn initial_prefers_sound() -> bool {
    use wasm_bindgen::JsCast;

    let doc = document().unchecked_into::<web_sys::HtmlDocument>();
    let cookie = doc.cookie().unwrap_or_default();
    cookie.contains("sound=true")
}

#[cfg(feature = "ssr")]
fn initial_prefers_sound() -> bool {
    use_context::<actix_web::HttpRequest>()
        .and_then(|req| {
            req.cookies()
                .map(|cookies| {
                    cookies
                        .iter()
                        .any(|cookie| cookie.name() == "sound" && cookie.value() == "true")
                })
                .ok()
        })
        .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq)]
pub enum SoundType {
    Turn,
    NewGame,
    LowTime,
}

#[derive(Clone)]
pub struct SoundsSignal {
    pub action: Action<ToggleSound, Result<bool, ServerFnError>>,
    pub context: RwSignal<Option<AudioContext>>,
    pub prefers_sound: Signal<bool>,
    pub turn: RwSignal<Option<AudioBuffer>>,
    pub low: RwSignal<Option<AudioBuffer>>,
    pub new: RwSignal<Option<AudioBuffer>>,
}

impl SoundsSignal {
    pub fn play_sound(&self, kind: SoundType) {
        if !self.prefers_sound.get() {
            return;
        };
        if let Some(context) = self.context.get() {
            let (signal, offset, duration) = match kind {
                SoundType::Turn => (self.turn, rand::thread_rng().gen_range(0..20) as f64, 500.0),
                SoundType::NewGame => (self.new, 0.0, 0.0),
                SoundType::LowTime => (self.low, 0.0, 0.0),
            };
            let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
                move |sound: AudioBufferSourceNode| {
                    let _ = sound.stop();
                },
                duration,
            );
            if let Some(buffer) = signal.get_untracked() {
                let source = context.create_buffer_source().unwrap();
                source.set_buffer(Some(&buffer));
                source.set_loop(false);
                source.start_with_when(offset).unwrap();
                source
                    .connect_with_audio_node(&context.destination())
                    .unwrap();
                if kind == SoundType::Turn {
                    start(source);
                }
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

pub fn provide_sounds() -> Signal<bool> {
    let context = RwSignal::new(None);
    let turn = RwSignal::new(None);
    let low = RwSignal::new(None);
    let new = RwSignal::new(None);
    let initial = initial_prefers_sound();

    let toggle_sound_action = create_server_action::<ToggleSound>();
    // input is Some(value) when pending, and None if not pending
    let input = toggle_sound_action.input();
    // value contains most recently-returned value
    let value = toggle_sound_action.value();

    let prefers_sound_fn = move || {
        match (input(), value()) {
            // if there's some current input, use that optimistically
            (Some(submission), _) => submission.prefers_sound,
            // otherwise, if there was a previous value confirmed by server, use that
            (_, Some(Ok(value))) => value,
            // otherwise, use the initial value
            _ => initial,
        }
    };
    let prefers_sound = Signal::derive(prefers_sound_fn);
    provide_context(SoundsSignal {
        action: toggle_sound_action,
        context,
        prefers_sound,
        turn,
        low,
        new,
    });
    prefers_sound
}
