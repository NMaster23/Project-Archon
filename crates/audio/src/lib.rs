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
use any_tts::{load_model, ModelType, SynthesisRequest, TtsConfig, TtsModel};
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};

pub fn stt(
    tx_out: tokio::sync::mpsc::UnboundedSender<TalosBus>,
    speaking: Arc<AtomicBool>,
    stt_disabled: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = StreamingModel::load(
        &PathBuf::from("models/moonshine-streaming-medium-onnx"),
        4,
        &Quantization::default(),
    )
        .map_err(|e| e.to_string())?;
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no output device available")?;
    let config: cpal::StreamConfig = device.default_input_config().map_err(|e| e.to_string())?.into();
    let sample_rate = config.sample_rate as f64;
    let channels = config.channels as usize;
    let params = InterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: InterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        16000.0 / sample_rate,
        2.0,
        params,
        1024,
        1,
    )?;
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
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let mut audio = Vec::new();
    let mut speech_buffer = Vec::new();
    let mut silence_chunks = 0;
    let mut vad_buffer: Vec<f32> = Vec::new();
    let mut vad = Vad::new_with_rate_and_mode(Rate16kHz, Quality);
    while let Ok(samples) = rx.recv() {
        for frame in samples.chunks_exact(channels) {
            let sample = frame.iter().sum::<f32>() / channels as f32;
            audio.push(sample);
        }
        if speaking.load(Ordering::Relaxed) || stt_disabled.load(Ordering::Relaxed) {
            audio.clear();
            vad_buffer.clear();
            speech_buffer.clear();
            silence_chunks = 0;
            continue;
        }
        let chunk_size = 1024;
        while audio.len() >= chunk_size {
            let chunk: Vec<f32> = audio.drain(..chunk_size).collect();
            let waves_in = vec![chunk];
            let mut waves_out = resampler.process(&waves_in, None)?;
            let chunk_16k = waves_out.remove(0);
            vad_buffer.extend(chunk_16k);
            while vad_buffer.len() >= 480 {
                let vad_chunk: Vec<f32> = vad_buffer.drain(..480).collect();
                let pcm16: Vec<i16> = vad_chunk
                    .iter()
                    .map(|&x| (x * i16::MAX as f32) as i16)
                    .collect();
                let is_talking = vad.is_voice_segment(&pcm16).unwrap_or(false);
                if is_talking {
                    speech_buffer.extend(&vad_chunk);
                    silence_chunks = 0;
                } else {
                    silence_chunks += 1;
                    if !speech_buffer.is_empty() {
                        speech_buffer.extend(&vad_chunk);
                    }
                }
                if silence_chunks >= 20 && !speech_buffer.is_empty() {
                    let result = model.transcribe(&speech_buffer, &TranscribeOptions::default());
                    if let Ok(res) = result {
                        let result_clone = res.text.clone();
                        let transcript = result_clone.clone();
                        if !result_clone.is_empty()
                            && tx_out.send(TalosBus::VoiceTranscript(transcript)).is_err()
                        {
                            break;
                        }
                    }
                    speech_buffer.clear();
                }
            }
        }
    }
    Ok(())
}

pub async fn tts(text: &str, model: &Box<dyn TtsModel>) -> Result<(), Box<dyn std::error::Error>> {
    let audio = model.synthesize(
        &SynthesisRequest::new(text)
            .with_language("English")
            .with_voice("Ryan")
            .with_instruct("Calm, clear, slightly upbeat."),
    )?;
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");
    let config: cpal::StreamConfig = device.default_output_config().map_err(|e| e.to_string())?.into();
    let channels = config.channels as usize;
    let sample_rate = audio.sample_rate;
    let mut current_idx = 0;
    let audio_len = audio.len();
    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let sample_val = if current_idx < audio_len {
                    let samples = audio.samples[current_idx];
                    current_idx += 1;
                    samples
                } else {
                    0.0
                };
                for sample in frame.iter_mut() {
                    *sample = sample_val;
                }
            }
        },
        move |err| {
            println!("an error occurred on the input stream: {}", err);
        },
        None,
    )?;
    stream.play()?;
    let duration_secs = audio_len as f32 / sample_rate as f32;
    tokio::time::sleep(std::time::Duration::from_secs_f32(duration_secs)).await;
    Ok(())
}