use freya::prelude::*;
use crate::setting_page::{AppSettings, AudioFormat};
use crate::effect;

use chrono::Local;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use flacenc::{
    config::Encoder as FlacEncoder, 
    source::MemSource, 
    bitsink::ByteSink, 
    component::BitRepr,
    error::Verify
};
use std::{
    sync::{Arc, Mutex},
    thread,
};

#[derive(Clone)]
struct RecordingDevice {
    device_index: usize,
    device_name: String,
    is_recording: bool,
    waveform_data: Arc<Mutex<Vec<f32>>>,
    recording_start_time: Option<std::time::Instant>,
}

#[derive(Clone)]
struct AppState {
    recording_devices: Vec<RecordingDevice>,
    input_devices: Vec<(String, usize)>,
}

impl AppState {
    fn new() -> Self {
        let host = cpal::default_host();
        let input_devices: Vec<(String, usize)> = host
            .input_devices()
            .unwrap()
            .enumerate()
            .filter_map(|(i, device)| {
                device.name().ok().map(|name| {
                    let display_name = if name.is_empty() {
                        format!("入力デバイス {}", i + 1)
                    } else {
                        name
                    };
                    (display_name, i)
                })
            })
            .collect();

        Self {
            recording_devices: vec![],
            input_devices,
        }
    }
}

#[component]
fn RecordingButton(
    device_idxs: Vec<usize>,
    app_state: Signal<AppState>,
    recorder_handles: Signal<Vec<Option<thread::JoinHandle<()>>>>,
    stop_flags: Signal<Vec<Arc<Mutex<bool>>>>,
) -> Element {
    let any_recording = device_idxs.iter().any(|&idx| {
        idx < app_state.read().recording_devices.len()
            && app_state.read().recording_devices[idx].is_recording
    });

    rsx! {
        FilledButton {
            onpress: {
                to_owned![device_idxs, app_state, recorder_handles, stop_flags];
                move |_| {
                    let is_any_recording = device_idxs.iter().any(|&idx| {
                        idx < app_state.read().recording_devices.len() &&
                        app_state.read().recording_devices[idx].is_recording
                    });

                    if !is_any_recording {
                        // 全デバイスの録音開始
                        for &device_idx in &device_idxs {
                            if device_idx < app_state.read().recording_devices.len() {
                                app_state.write().recording_devices[device_idx].is_recording = true;
                                app_state.write().recording_devices[device_idx].recording_start_time = Some(std::time::Instant::now());

                                if device_idx < stop_flags.read().len() {
                                    *stop_flags.read()[device_idx].lock().unwrap() = false;
                                }

                                let selected_device_index = app_state.read().recording_devices[device_idx].device_index;
                                let device_name = app_state.read().recording_devices[device_idx].device_name.clone();
                                let stop_flag_clone = if device_idx < stop_flags.read().len() {
                                    stop_flags.read()[device_idx].clone()
                                } else {
                                    Arc::new(Mutex::new(false))
                                };
                                let waveform_data_clone = app_state.read().recording_devices[device_idx].waveform_data.clone();

                                let handle = thread::spawn(move || {
                                    let settings = AppSettings::load();
                                    let host = cpal::default_host();
                                    let device = host
                                        .input_devices()
                                        .unwrap()
                                        .nth(selected_device_index)
                                        .expect("選択されたデバイスが見つかりません");
                                    let config = device.default_input_config().unwrap();

                                    let now = Local::now();
                                    let (filename, writer_opt) = match settings.audio_format {
                                        AudioFormat::Wave => {
                                            let spec = WavSpec {
                                                channels: config.channels(),
                                                sample_rate: settings.sample_rate,
                                                bits_per_sample: settings.bit_depth,
                                                sample_format: hound::SampleFormat::Int,
                                            };
                                            let filename = format!("{}-{}.wav",
                                                now.format("%Y-%m-%d-%H-%M-%S"),
                                                device_name.replace(" ", "_")
                                            );
                                            let writer = WavWriter::create(&filename, spec).unwrap();
                                            (filename, Some(Arc::new(Mutex::new(Some(writer)))))
                                        },
                                        AudioFormat::Pcm => {
                                            let filename = format!("{}-{}.pcm",
                                                now.format("%Y-%m-%d-%H-%M-%S"),
                                                device_name.replace(" ", "_")
                                            );
                                            (filename, None)
                                        },
                                        AudioFormat::Flac => {
                                            let filename = format!("{}-{}.flac",
                                                now.format("%Y-%m-%d-%H-%M-%S"),
                                                device_name.replace(" ", "_")
                                            );
                                            (filename, None)
                                        },
                                    };

                                    let pcm_file = if matches!(settings.audio_format, AudioFormat::Pcm) {
                                        Some(Arc::new(Mutex::new(std::fs::File::create(&filename).unwrap())))
                                    } else {
                                        None
                                    };

                                    // FLAC用のサンプルバッファ
                                    let flac_samples = if matches!(settings.audio_format, AudioFormat::Flac) {
                                        Some(Arc::new(Mutex::new(Vec::<i32>::new())))
                                    } else {
                                        None
                                    };

                                    let err_fn = |err| eprintln!("録音エラー: {:?}", err);
                                    let writer_clone = writer_opt.clone();
                                    let pcm_file_clone = pcm_file.clone();
                                    let flac_samples_clone = flac_samples.clone();
                                    let stop_flag_stream = Arc::clone(&stop_flag_clone);
                                    let waveform_clone = waveform_data_clone.clone();
                                    let format = settings.audio_format.clone();
                                    
                                    // コンプレッサー設定をローカル変数にコピー
                                    let compressor_enabled = settings.compressor_enabled;
                                    // let compressor_threshold_db = settings.compressor_threshold_db;
                                    // let compressor_ratio = settings.compressor_ratio;
                                    let compressor_threshold_db:f32 = -20.0;
                                    let compressor_ratio:f32 = 4.0;

                                    let stream = match config.sample_format() {
                                        cpal::SampleFormat::F32 => device.build_input_stream(
                                            &config.into(),
                                            move |data: &[f32], _| {
                                                if *stop_flag_stream.lock().unwrap() {
                                                    return;
                                                }

                                                // コンプレッサーを適用（設定で有効な場合）
                                                let processed_data = if compressor_enabled {
                                                    effect::compress_audio(
                                                        data,
                                                        compressor_threshold_db,
                                                        compressor_ratio
                                                    )
                                                } else {
                                                    data.to_vec()
                                                };

                                                // 波形データを更新
                                                {
                                                    let mut waveform = waveform_clone.lock().unwrap();
                                                    waveform.clear();
                                                    waveform.extend_from_slice(&processed_data);
                                                    if waveform.len() > 300 {
                                                        let len = waveform.len();
                                                        waveform.drain(0..len-300);
                                                    }
                                                }

                                                // フォーマットに応じてデータを書き込み
                                                match format {
                                                    AudioFormat::Wave => {
                                                        if let Some(ref writer_arc) = writer_clone {
                                                            let mut writer_lock = writer_arc.lock().unwrap();
                                                            if let Some(writer) = writer_lock.as_mut() {
                                                                for &sample in &processed_data {
                                                                    let sample_i16 = (sample * i16::MAX as f32) as i16;
                                                                    writer.write_sample(sample_i16).unwrap();
                                                                }
                                                            }
                                                        }
                                                    },
                                                    AudioFormat::Pcm => {
                                                        if let Some(ref pcm_file_arc) = pcm_file_clone {
                                                            use std::io::Write;
                                                            let mut file = pcm_file_arc.lock().unwrap();
                                                            for &sample in &processed_data {
                                                                let sample_i16 = (sample * i16::MAX as f32) as i16;
                                                                file.write_all(&sample_i16.to_le_bytes()).unwrap();
                                                            }
                                                        }
                                                    },
                                                    AudioFormat::Flac => {
                                                        if let Some(ref flac_samples_arc) = flac_samples_clone {
                                                            let mut samples = flac_samples_arc.lock().unwrap();
                                                            for &sample in &processed_data {
                                                                let sample_i32 = (sample * i32::MAX as f32) as i32;
                                                                samples.push(sample_i32);
                                                            }
                                                        }
                                                    },
                                                }
                                            },
                                            err_fn,
                                            None,
                                        ),
                                        _ => panic!("対応していないサンプル形式"),
                                    }.unwrap();

                                    stream.play().unwrap();
                                    while !*stop_flag_clone.lock().unwrap() {
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                    }

                                    // フォーマットに応じてファイルを閉じる
                                    match settings.audio_format {
                                        AudioFormat::Wave => {
                                            if let Some(writer_arc) = writer_opt {
                                                writer_arc.lock().unwrap().take().unwrap().finalize().unwrap();
                                            }
                                        },
                                        AudioFormat::Pcm => {
                                            // PCMファイルは自動的に閉じられる
                                        },
                                        AudioFormat::Flac => {
                                            if let Some(flac_samples_arc) = flac_samples {
                                                let samples = flac_samples_arc.lock().unwrap();
                                                if !samples.is_empty() {
                                                    // FLACエンコーディング
                                                    let config = FlacEncoder::default().into_verified().unwrap();
                                                    let source = MemSource::from_samples(
                                                        &samples,
                                                        1,  // モノラルとして扱う
                                                        settings.bit_depth as usize,
                                                        settings.sample_rate as usize,
                                                    );
                                                    
                                                    match flacenc::encode_with_fixed_block_size(
                                                        &config, source, config.block_size
                                                    ) {
                                                        Ok(flac_stream) => {
                                                            let mut sink = ByteSink::new();
                                                            if flac_stream.write(&mut sink).is_ok() {
                                                                if let Err(e) = std::fs::write(&filename, sink.as_slice()) {
                                                                    eprintln!("FLACファイル書き込みエラー: {}", e);
                                                                }
                                                            } else {
                                                                eprintln!("FLACストリーム書き込みエラー");
                                                            }
                                                        },
                                                        Err(e) => eprintln!("FLACエンコードエラー: {}", e),
                                                    }
                                                }
                                            }
                                        },
                                    }
                                });

                                if device_idx < recorder_handles.read().len() {
                                    recorder_handles.write()[device_idx] = Some(handle);
                                }
                            }
                        }
                    } else {
                        // 全デバイスの録音停止
                        for &device_idx in &device_idxs {
                            if device_idx < app_state.read().recording_devices.len() {
                                app_state.write().recording_devices[device_idx].is_recording = false;
                                app_state.write().recording_devices[device_idx].recording_start_time = None;

                                if device_idx < stop_flags.read().len() {
                                    *stop_flags.read()[device_idx].lock().unwrap() = true;
                                }

                                if device_idx < recorder_handles.read().len() {
                                    if let Some(handle) = recorder_handles.write()[device_idx].take() {
                                        handle.join().unwrap();
                                    }
                                }
                            }
                        }
                    }
                }
            },
            label {
                if any_recording {
                    "⏹️ 録音停止"
                } else {
                    "🔴 録音開始"
                }
            }
        }
    }
}
#[component]
pub fn record_page(on_navigate_to_settings: EventHandler<()>) -> Element {
    let mut app_state = use_signal(|| AppState::new());
    let mut recorder_handles: Signal<Vec<Option<thread::JoinHandle<()>>>> =
        use_signal(|| Vec::new());
    let mut stop_flags: Signal<Vec<Arc<Mutex<bool>>>> = use_signal(|| Vec::new());

    rsx! {
            rect {
            background: "rgb(40, 44, 52)",
            ScrollView {
                width: "100%",
                height: "100%",
                direction: "vertical",

                rect {
                    width: "100%",
                    height: "auto",
                    direction: "vertical",
                    padding: "20",

                    // タイトル
                    label {
                        color: "white",
                        font_size: "38",
                        text_align: "center",
                        "録音くん"
                    }

                    rect { height: "20" }

                    // 録音時間表示
                    label {
                        color: "white",
                        font_size: "24",
                        text_align: "center",
                        {
                            let recording_devices = app_state.read();
                            if let Some(device) = recording_devices.recording_devices.iter().find(|d| d.is_recording) {
                                if let Some(start_time) = device.recording_start_time {
                                    let elapsed = start_time.elapsed().as_secs();
                                    format!("録音時間: {:02}:{:02}", elapsed / 60, elapsed % 60)
                                } else {
                                    "録音時間: 00:00".to_string()
                                }
                            } else {
                                "録音時間: 00:00".to_string()
                            }
                        }
                    }

                    rect { height: "20" }
                    rect {
                    direction: "horizontal",
                    main_align: "center",
                    cross_align: "center",

                    // プラスボタンでデバイス追加
                    FilledButton {
                        onpress: move |_| {
                            if !app_state.read().input_devices.is_empty() {
                                let device_index = 0;
                                let device_name = app_state.read().input_devices[0].0.clone();

                                app_state.write().recording_devices.push(RecordingDevice {
                                    device_index,
                                    device_name,
                                    is_recording: false,
                                    waveform_data: Arc::new(Mutex::new(vec![0.0; 200])),
                                    recording_start_time: None,
                                });

                                recorder_handles.write().push(None);
                                stop_flags.write().push(Arc::new(Mutex::new(false)));
                            }
                        },
                        label { "➕ マイクを追加" }
                    }

                    rect { width: "20" }
                    Button {
                        onpress: move |_| on_navigate_to_settings.call(()),
                        label { "⚙️ 設定ページへ" }
                    }

                    rect { width: "20" }

                    // 全デバイス同時録音ボタン
                    if !app_state.read().recording_devices.is_empty() {
                        RecordingButton {
                            device_idxs: (0..app_state.read().recording_devices.len()).collect::<Vec<_>>(),
                            app_state: app_state,
                            recorder_handles: recorder_handles,
                            stop_flags: stop_flags,
                        }
                    }
                    }

                    rect { height: "30" }

                    // マイクデバイスリスト（スクロール可能コンテナ）
                    rect {
                        width: "100%",
                        height: "auto",
                        direction: "vertical",

                        for (device_idx, recording_device) in app_state.read().recording_devices.iter().enumerate() {
                        rect {
                            width: "100%",
                            height: "auto",
                            background: "rgb(60, 64, 72)",
                            border: "2 solid rgb(100, 100, 100)",
                            corner_radius: "8",
                            padding: "20",
                            margin: "10",
                            direction: "vertical",

                            // デバイス情報とコントロール
                            rect {
                                direction: "horizontal",
                                main_align: "space-between",
                                cross_align: "center",
                                width: "100%",

                                // デバイス選択
                                rect {
                                    direction: "horizontal",
                                    cross_align: "center",

                                    label {
                                        color: "white",
                                        font_size: "16",
                                        "デバイス: "
                                    }

                                    Dropdown {
                                        value: recording_device.device_name.clone(),

                                        for (i, (name, _)) in app_state.read().input_devices.iter().enumerate() {
                                            DropdownItem {
                                                value: i.to_string(),
                                                onpress: {
                                                    to_owned![device_idx, i, name];
                                                    move |_| {
                                                        if device_idx < app_state.read().recording_devices.len() {
                                                            app_state.write().recording_devices[device_idx].device_index = i;
                                                            app_state.write().recording_devices[device_idx].device_name = name.clone();
                                                        }
                                                    }
                                                },
                                                label { "{name}" }
                                            }
                                        }
                                    }
                                }

                                // 削除ボタン
                                rect {
                                    direction: "horizontal",
                                    cross_align: "center",

                                    Button {
                                        onpress: {
                                            to_owned![device_idx];
                                            move |_| {
                                                if device_idx < app_state.read().recording_devices.len() {
                                                    // 録音中の場合は先に停止
                                                    if app_state.read().recording_devices[device_idx].is_recording {
                                                        if device_idx < stop_flags.read().len() {
                                                            *stop_flags.read()[device_idx].lock().unwrap() = true;
                                                        }
                                                        if device_idx < recorder_handles.read().len() {
                                                            if let Some(handle) = recorder_handles.write()[device_idx].take() {
                                                                handle.join().unwrap();
                                                            }
                                                        }
                                                    }

                                                    app_state.write().recording_devices.remove(device_idx);
                                                    recorder_handles.write().remove(device_idx);
                                                    stop_flags.write().remove(device_idx);
                                                }
                                            }
                                        },
                                        label { "🗑️ 削除" }
                                    }
                                }
                            }

                            rect { height: "10" }

                            // 波形表示
                            rect {
                                width: "100%",
                                height: "120",
                                background: "rgb(30, 30, 30)",
                                border: "1 solid rgb(100, 100, 100)",
                                corner_radius: "4",
                                direction: "horizontal",
                                main_align: "start",
                                cross_align: "center",
                                overflow: "clip",

                                // 波形データを表示（録音中でなくても表示）
                                {
                                    let waveform_data = recording_device.waveform_data.lock().unwrap();
                                    let data_len = waveform_data.len();

                                    if data_len > 0 {
                                        // データがある場合は波形を表示
                                        let step = if data_len > 200 { data_len / 200 } else { 1 };
                                        rsx! {
                                            for (_, sample) in waveform_data.iter().step_by(step).enumerate() {
                                                rect {
                                                    width: "2",
                                                    height: "{(sample.abs() * 100.0).max(2.0).min(110.0)}",
                                                    background: if recording_device.is_recording { "rgb(0, 255, 0)" } else { "rgb(100, 150, 255)" },
                                                    margin: "0 1",
                                                }
                                            }
                                        }
                                    } else {
                                        // データがない場合はフラットライン
                                        rsx! {
                                            for _ in 0..100 {
                                                rect {
                                                    width: "2",
                                                    height: "2",
                                                    background: "rgb(80, 80, 80)",
                                                    margin: "0 1",
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if recording_device.is_recording {
                                rect { height: "5" }
                                label {
                                    color: "red",
                                    font_size: "14",
                                    "🔴 録音中..."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    }
}
