use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use transcribe_rs::models::StreamingModel;
use transcribe_rs::quantization::Quantization;
use transcribe_rs::options::TranscribeOptions;
use talos_core::TalosBus;

pub fn stt(tx_out: tokio::sync::mpsc::UnboundedSender<TalosBus>, speaking: Arc<AtomicBool>) {
    let mut model = StreamingModel::load(
        &PathBuf::from("models\\moonshine-streaming-small-onnx"),
        4,
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
            println!("an error occurred on the input stream: {}", err);
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
            if speaking.load(Ordering::Relaxed) {
                audio.clear();
            }
            let filtered: Vec<f32> = audio.iter().step_by(skip).copied().collect();
            let rms = (filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32).sqrt();
            if rms > 0.002 {
                let result = model.transcribe(&filtered, &TranscribeOptions::default()).unwrap();
                if !result.text.is_empty() {
                    if tx_out.send(result.text.clone()).is_err() {
                        break;
                    }
                    println!("{:?}", result);
                }
            }
            audio.drain(..sample_rate);
        }
    }
}
