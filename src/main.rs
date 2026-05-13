use transcribe_rs::onnx::moonshine::StreamingModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::SpeechModel;
use std::path::PathBuf;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::Duration;
use std::thread;
use std::sync::mpsc::channel;
use transcribe_rs::TranscribeOptions;


fn stt() {
    let mut model = StreamingModel::load(
        &PathBuf::from("models\\moonshine-streaming-small-onnx"),
        4,  // threads
        &Quantization::default(),
    ).unwrap();
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no output device available");
    let mut supported_configs_range = device.supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range.next()
        .expect("no supported config?!")
        .with_max_sample_rate();
    let config = supported_config.into();
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            tx.send(data.to_vec()).ok();
        },
        move |err| {
        },
        None
    ).expect("failed to build stream");
    stream.play().unwrap();
    let mut audio = Vec::new();
    loop {
        while let Ok(samples) = rx.try_recv() {
            let skip = (config.sample_rate / 16000) as usize;
            let filtered: Vec<f32> = samples.into_iter().step_by(skip).collect();
            audio.extend(filtered);
        }
        if audio.len() > 16000 {
            let result = model.transcribe(&audio, &TranscribeOptions::default()).unwrap();
            if !result.text.is_empty() {
                println!("{:?}", result);
            }
            audio.clear();
        }
    }
}

fn main() {
    stt();
}