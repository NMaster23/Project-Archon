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
    println!("{}", device.description().unwrap());
    let config = device.default_input_config().unwrap().into();
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).ok();
        },
        move |err| {
        },
        None
    ).expect("failed to build stream");
    stream.play().unwrap();
    let mut audio = Vec::new();
    while let Ok(samples) = rx.recv() {
        let sample_rate = config.sample_rate as u32 as usize;
        let channels = config.channels as usize;
        for frame in samples.chunks_exact(channels) {
            let sample = frame.iter().sum::<f32>() / channels as f32;
            audio.push(sample);
        }
        
        if audio.len() >= sample_rate {
            let skip = (sample_rate / 16000).max(1) as usize;
            let filtered: Vec<f32> = audio.iter().step_by(skip).copied().collect();
            let rms = (filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32).sqrt();
            if rms > 0.002 {
                let result = model.transcribe(&filtered, &TranscribeOptions::default()).unwrap();
                if !result.text.is_empty() && result.text != "Thank you." {
                    println!("{:?}", result);
                }
            }
            audio.drain(..sample_rate);
        }
    }
}

fn main() {
    stt();
}