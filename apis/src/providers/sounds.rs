use super::Config;
use leptos::prelude::*;
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
struct ClientData {
    ctx: AudioContext,
    turn: AudioBuffer,
    low: AudioBuffer,
    new: AudioBuffer,
}
#[derive(Clone)]
pub struct Sounds {
    client_data: LocalResource<Result<ClientData, JsValue>>,
}

impl Sounds {
    pub fn play_sound(&self, kind: SoundType) {
        let config = expect_context::<Config>().0;
        if !config().prefers_sound {
            return;
        };
        if let Some(Ok(s)) = self.client_data.get() {
            let (buffer, offset, duration) = match kind {
                SoundType::Turn => (&s.turn, rand::thread_rng().gen_range(0..20) as f64, 1.0),
                SoundType::NewGame => (&s.new, 0.0, 3.0),
                SoundType::LowTime => (&s.low, 0.0, 2.0),
            };
            let source = s.ctx.create_buffer_source().unwrap();
            source.set_buffer(Some(buffer));
            source.set_loop(false);
            source
                .start_with_when_and_grain_offset_and_grain_duration(0.0, offset, duration)
                .unwrap();
            source
                .connect_with_audio_node(&(s.ctx.destination()))
                .unwrap();
        }
    }
}

async fn load_audio_buffer(ctx: &AudioContext, url: &str) -> Result<AudioBuffer, JsValue> {
    let f: JsFuture = window().fetch_with_str(url).into();
    let f: JsFuture = f.await?.dyn_into::<Response>()?.array_buffer()?.into();
    JsFuture::from(ctx.decode_audio_data(&f.await?.dyn_into::<ArrayBuffer>()?)?)
        .await?
        .dyn_into::<AudioBuffer>()
}

async fn load_client_data() -> Result<ClientData, JsValue> {
    let ctx = AudioContext::new()?;
    let low = load_audio_buffer(&ctx, "/assets/low.mp3").await?;
    let new = load_audio_buffer(&ctx, "/assets/new.mp3").await?;
    let turn = load_audio_buffer(&ctx, "/assets/moves.mp3").await?;
    Ok(ClientData {
        ctx,
        turn,
        low,
        new,
    })
}

pub fn provide_sounds() {
    provide_context(Sounds {
        client_data: LocalResource::new(|| load_client_data()),
    });
}
