use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use talos_core::TalosBus;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::onnx::moonshine::StreamingModel;
use transcribe_rs::{SpeechModel, TranscribeOptions};
use webrtc_vad::SampleRate::Rate16kHz;
use webrtc_vad::VadMode::Quality;
use webrtc_vad::*;

pub fn stt(
    tx_out: tokio::sync::mpsc::UnboundedSender<TalosBus>,
    speaking: Arc<AtomicBool>,
    stt_disabled: Arc<AtomicBool>,
) {
    let mut model = StreamingModel::load(
        &PathBuf::from("models\\moonshine-streaming-medium-onnx"),
        4,
        &Quantization::default(),
    )
    .unwrap();
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("no output device available");
    let config = device.default_input_config().unwrap().into();
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    let stream = device
        .build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                tx.send(data.to_vec()).ok();
            },
            move |err| {
                println!("an error occurred on the input stream: {}", err);
            },
            None,
        )
        .expect("failed to build stream");
    stream.play().unwrap();
    let mut audio = Vec::new();
    let mut speech_buffer = Vec::new();
    let mut silence_chunks = 0;
    let mut vad = Vad::new_with_rate_and_mode(Rate16kHz, Quality);
    while let Ok(samples) = rx.recv() {
        let sample_rate = config.sample_rate as u32 as usize;
        let channels = config.channels as usize;
        for frame in samples.chunks_exact(channels) {
            let sample = frame.iter().sum::<f32>() / channels as f32;
            audio.push(sample);
            let chunk_size = (sample_rate * 30) / 1000;
            if speaking.load(Ordering::Relaxed) || stt_disabled.load(Ordering::Relaxed) {
                audio.clear();
                speech_buffer.clear();
            }
            if audio.len() >= chunk_size {
                let skip = (sample_rate / 16000).max(1);
                let chunk: Vec<f32> = audio.drain(..chunk_size).collect();
                let mut chunk_16k = Vec::new();
                for i in (0..chunk.len()).step_by(skip as usize) {
                    chunk_16k.push(chunk[i]);
                }
                chunk_16k.resize(480, 0.0);
                let pcm16: Vec<i16> = chunk_16k
                    .iter()
                    .map(|&x| (x * i16::MAX as f32) as i16)
                    .collect();
                let is_talking = vad.is_voice_segment(&pcm16).unwrap_or(false);
                if is_talking {
                    speech_buffer.extend(chunk_16k);
                    silence_chunks = 0;
                } else {
                    silence_chunks += 1;
                    if !speech_buffer.is_empty() {
                        speech_buffer.extend(chunk_16k);
                    }
                }
                if silence_chunks == 20 && !speech_buffer.is_empty() {
                    let result = model.transcribe(&speech_buffer, &TranscribeOptions::default());
                    let result_clone = result.unwrap().text.clone();
                    let transcript = result_clone.clone();
                    if !result_clone.is_empty()
                        && tx_out.send(TalosBus::VoiceTranscript(transcript)).is_err()
                    {
                        break;
                    }
                    speech_buffer.clear();
                }
            }
        }
    }
}
