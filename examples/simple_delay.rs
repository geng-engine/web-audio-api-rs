use std::fs::File;
use web_audio_api::context::{
    AudioContext, AudioContextLatencyCategory, AudioContextOptions, BaseAudioContext,
};
use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};

// DelayNode example
//
// `cargo run --release --example simple_delay`
//
// If you are on Linux and use ALSA as audio backend backend, you might want to run
// the example with the `WEB_AUDIO_LATENCY=playback ` env variable which will
// increase the buffer size to 1024
//
// `WEB_AUDIO_LATENCY=playback cargo run --release --example simple_delay`
fn main() {
    env_logger::init();

    let context = match std::env::var("WEB_AUDIO_LATENCY") {
        Ok(val) => {
            if val == "playback" {
                AudioContext::new(AudioContextOptions {
                    latency_hint: AudioContextLatencyCategory::Playback,
                    ..AudioContextOptions::default()
                })
            } else {
                println!("Invalid WEB_AUDIO_LATENCY value, fall back to default");
                AudioContext::default()
            }
        }
        Err(_e) => AudioContext::default(),
    };

    let file = File::open("samples/sample.wav").unwrap();
    let audio_buffer = context.decode_audio_data_sync(file).unwrap();

    // create a delay of 0.5s
    let delay = context.create_delay(1.);
    delay.delay_time().set_value(0.5);
    delay.connect(&context.destination());

    let src = context.create_buffer_source();
    src.set_buffer(audio_buffer);
    // connect to both delay and destination
    src.connect(&delay);
    src.connect(&context.destination());
    src.start();

    std::thread::sleep(std::time::Duration::from_millis(5000));
}
