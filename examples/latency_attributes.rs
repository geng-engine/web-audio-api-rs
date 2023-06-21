use web_audio_api::context::{
    AudioContext, AudioContextLatencyCategory, AudioContextOptions, BaseAudioContext,
};
use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};

// Retrieve the output latency of the audio context
//
// `cargo run --release --example latency_attributes`
//
// If you are on Linux and use ALSA as audio backend backend, you might want to run
// the example with the `WEB_AUDIO_LATENCY=playback ` env variable which will
// increase the buffer size to 1024
//
// `WEB_AUDIO_LATENCY=playback cargo run --release --example latency_attributes`
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

    let sine = context.create_oscillator();
    sine.frequency().set_value(200.);
    sine.connect(&context.destination());

    sine.start();

    println!("- BaseLatency: {:?}", context.base_latency());

    loop {
        println!("-------------------------------------------------");
        println!("+ currentTime {:?}", context.current_time());
        println!("+ OutputLatency: {:?}", context.output_latency());

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
