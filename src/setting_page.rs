use freya::prelude::*;
use nojson::{DisplayJson, Json, JsonFormatter, JsonParseError, RawJsonValue, json};
use std::fs;
use std::path::Path;
use dioxus_i18n::{prelude::*, t};
use crate::i18n::Language;

#[derive(Clone, PartialEq)]
pub struct AppSettings {
    pub audio_format: AudioFormat,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub compressor_enabled: bool,
    pub compressor_threshold_db: f32,
    pub compressor_ratio: f32,
    pub language: Language,
}

#[derive(Clone, PartialEq)]
pub enum AudioFormat {
    Wave,
    Pcm,
    Flac,
}

//設定項目の定義...?
impl DisplayJson for AppSettings {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> std::fmt::Result {
        f.object(|f| {
            f.member(
                "audio_format",
                match self.audio_format {
                    AudioFormat::Wave => "WAVE",
                    AudioFormat::Pcm => "PCM",
                    AudioFormat::Flac => "FLAC",
                },
            )?;
            f.member("sample_rate", self.sample_rate)?;
            f.member("bit_depth", self.bit_depth)?;
            f.member("compressor_enabled", self.compressor_enabled)?;
            f.member("compressor_threshold_db", self.compressor_threshold_db)?;
            f.member("compressor_ratio", self.compressor_ratio)?;
            f.member("language", match self.language {
                Language::Japanese => "ja",
                Language::English => "en",
            })
        })
    }
}

//設定項目の定義
impl<'text, 'raw> TryFrom<RawJsonValue<'text, 'raw>> for AppSettings {
    type Error = JsonParseError;

    fn try_from(value: RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let audio_format_str: String = value.to_member("audio_format")?.required()?.try_into()?;
        let audio_format = match audio_format_str.as_str() {
            "WAVE" => AudioFormat::Wave,
            "PCM" => AudioFormat::Pcm,
            "FLAC" => AudioFormat::Flac,
            _ => return Err(value.invalid("Invalid audio format")),
        };

        let sample_rate = value.to_member("sample_rate")?.required()?.try_into()?;
        let bit_depth = value.to_member("bit_depth")?.required()?.try_into()?;
        
        // コンプレッサー設定（オプション、デフォルト値あり）
        let compressor_enabled = match value.to_member("compressor_enabled") {
            Ok(member) => match member.required() {
                Ok(val) => val.try_into().unwrap_or(false),
                Err(_) => false,
            },
            Err(_) => false,
        };
        let compressor_threshold_db = match value.to_member("compressor_threshold_db") {
            Ok(member) => match member.required() {
                Ok(val) => val.try_into().unwrap_or(-20.0),
                Err(_) => -20.0,
            },
            Err(_) => -20.0,
        };
        let compressor_ratio = match value.to_member("compressor_ratio") {
            Ok(member) => match member.required() {
                Ok(val) => val.try_into().unwrap_or(4.0),
                Err(_) => 4.0,
            },
            Err(_) => 4.0,
        };

        // 言語設定（オプション、デフォルト値あり）
        let language = match value.to_member("language") {
            Ok(member) => match member.required() {
                Ok(val) => {
                    let lang_str: String = val.try_into().unwrap_or("ja".to_string());
                    match lang_str.as_str() {
                        "en" => Language::English,
                        _ => Language::Japanese,
                    }
                },
                Err(_) => Language::Japanese,
            },
            Err(_) => Language::Japanese,
        };

        Ok(AppSettings {
            audio_format,
            sample_rate,
            bit_depth,
            compressor_enabled,
            compressor_threshold_db,
            compressor_ratio,
            language,
        })
    }
}

//デフォルト設定
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            audio_format: AudioFormat::Wave,
            sample_rate: 44100,
            bit_depth: 16,
            compressor_enabled: false,
            compressor_threshold_db: -20.0,
            compressor_ratio: 4.0,
            language: Language::Japanese,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        if Path::new("settings.json").exists() {
            match fs::read_to_string("settings.json") {
                Ok(content) => match content.parse::<Json<AppSettings>>() {
                    Ok(settings) => settings.0,
                    Err(_) => Self::default(),
                },
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json_content = json(|f| {
            f.set_indent_size(2);
            f.set_spacing(true);
            f.value(self)
        })
        .to_string();

        fs::write("settings.json", json_content)?;
        Ok(())
    }
}

#[component]
pub fn SettingsPage(on_navigate_to_recording: EventHandler<()>) -> Element {
    let mut settings = use_signal(|| AppSettings::load());
    let mut save_message = use_signal(|| String::new());
    let mut i18n = i18n();

    // 言語が変更されたら、i18nの言語も更新
    use_effect(move || {
        let current_language = settings.read().language;
        match current_language {
            Language::Japanese => i18n.set_language("ja".parse().unwrap_or_default()),
            Language::English => i18n.set_language("en".parse().unwrap_or_default()),
        };
    });

    rsx! {
        rect {
            width: "100%",
            height: "100%",
            background: "rgb(40, 44, 52)",
            direction: "vertical",

            ScrollView {
                width: "100%",
                height: "calc(100% - 80)",
                direction: "vertical",

                rect {
                    width: "100%",
                    height: "auto",
                    padding: "20",
                    direction: "vertical",

                label {
                    color: "white",
                    font_size: "28",
                    "{t!(\"settings_title\")}"
                }

                rect { height: "30" }

                // 音声フォーマット設定
                rect {
                    width: "100%",
                    height: "auto",
                    direction: "vertical",
                    background: "rgb(60, 64, 72)",
                    border: "1 solid rgb(100, 100, 100)",
                    corner_radius: "8",
                    padding: "20",
                    margin: "10 0",

                    label {
                        color: "white",
                        font_size: "20",
                        "{t!(\"audio_format_section\")}"
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "{t!(\"save_format\")}: "
                        }

                        Dropdown {
                            value: match settings.read().audio_format {
                                AudioFormat::Wave => "WAVE",
                                AudioFormat::Pcm => "PCM",
                                AudioFormat::Flac => "FLAC",
                            },

                            DropdownItem {
                                value: "WAVE",
                                onpress: move |_| {
                                    settings.write().audio_format = AudioFormat::Wave;
                                },
                                label { "WAVE" }
                            }

                            DropdownItem {
                                value: "PCM",
                                onpress: move |_| {
                                    settings.write().audio_format = AudioFormat::Pcm;
                                },
                                label { "PCM" }
                            }

                            DropdownItem {
                                value: "FLAC",
                                onpress: move |_| {
                                    settings.write().audio_format = AudioFormat::Flac;
                                },
                                label { "FLAC(使用不可)" }
                            }
                        }
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "サンプルレート: "
                        }

                        Dropdown {
                            value: format!("{}", settings.read().sample_rate),

                            DropdownItem {
                                value: "44100",
                                onpress: move |_| {
                                    settings.write().sample_rate = 44100;
                                },
                                label { "44100 Hz" }
                            }

                            DropdownItem {
                                value: "48000",
                                onpress: move |_| {
                                    settings.write().sample_rate = 48000;
                                },
                                label { "48000 Hz" }
                            }

                            DropdownItem {
                                value: "96000",
                                onpress: move |_| {
                                    settings.write().sample_rate = 96000;
                                },
                                label { "96000 Hz" }
                            }
                        }
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "ビット深度: "
                        }

                        Dropdown {
                            value: format!("{}", settings.read().bit_depth),

                            DropdownItem {
                                value: "16",
                                onpress: move |_| {
                                    settings.write().bit_depth = 16;
                                },
                                label { "16 bit" }
                            }

                            DropdownItem {
                                value: "24",
                                onpress: move |_| {
                                    settings.write().bit_depth = 24;
                                },
                                label { "24 bit" }
                            }

                            DropdownItem {
                                value: "32",
                                onpress: move |_| {
                                    settings.write().bit_depth = 32;
                                },
                                label { "32 bit" }
                            }
                        }
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "言語: "
                        }

                        Dropdown {
                            value: match settings.read().language {
                                Language::Japanese => "ja",
                                Language::English => "en",
                            },

                            DropdownItem {
                                value: "ja",
                                onpress: move |_| {
                                    settings.write().language = Language::Japanese;
                                },
                                label { "日本語" }
                            }

                            DropdownItem {
                                value: "en",
                                onpress: move |_| {
                                    settings.write().language = Language::English;
                                },
                                label { "English" }
                            }
                        }
                    }
                }

                rect {
                    width: "100%",
                    height: "auto",
                    direction: "vertical",
                    background: "rgb(60, 64, 72)",
                    border: "1 solid rgb(100, 100, 100)",
                    corner_radius: "8",
                    padding: "20",
                    margin: "10 0",

                    label {
                        color: "white",
                        font_size: "20",
                        "後処理設定"
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "コンプレッサー: "
                        }

                        rect {
                            background: if settings.read().compressor_enabled { "rgb(0, 120, 255)" } else { "rgb(80, 80, 80)" },
                            padding: "8",
                            corner_radius: "4",
                            
                            Button {
                                onpress: move |_| {
                                    let current_state = settings.read().compressor_enabled;
                                    settings.write().compressor_enabled = !current_state;
                                },
                                label { 
                                    if settings.read().compressor_enabled { "✓ 有効" } else { "無効" }
                                }
                            }
                        }
                    }

                //     if settings.read().compressor_enabled {
                //         rect { height: "15" }

                //         rect {
                //             direction: "horizontal",
                //             cross_align: "center",

                //             label {
                //                 color: "white",
                //                 font_size: "16",
                //                 width: "120",
                //                 "スレッショルド: "
                //             }

                //             Dropdown {
                //                 value: format!("{}", settings.read().compressor_threshold_db),

                //                 DropdownItem {
                //                     value: "-10.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_threshold_db = -10.0;
                //                     },
                //                     label { "-10 dB" }
                //                 }

                //                 DropdownItem {
                //                     value: "-20.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_threshold_db = -20.0;
                //                     },
                //                     label { "-20 dB" }
                //                 }

                //                 DropdownItem {
                //                     value: "-30.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_threshold_db = -30.0;
                //                     },
                //                     label { "-30 dB" }
                //                 }
                //             }
                //         }

                //         rect { height: "15" }

                //         rect {
                //             direction: "horizontal",
                //             cross_align: "center",

                //             label {
                //                 color: "white",
                //                 font_size: "16",
                //                 width: "120",
                //                 "レシオ: "
                //             }

                //             Dropdown {
                //                 value: format!("{}", settings.read().compressor_ratio),

                //                 DropdownItem {
                //                     value: "2.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_ratio = 2.0;
                //                     },
                //                     label { "2:1" }
                //                 }

                //                 DropdownItem {
                //                     value: "4.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_ratio = 4.0;
                //                     },
                //                     label { "4:1" }
                //                 }

                //                 DropdownItem {
                //                     value: "8.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_ratio = 8.0;
                //                     },
                //                     label { "8:1" }
                //                 }

                //                 DropdownItem {
                //                     value: "16.0",
                //                     onpress: move |_| {
                //                         settings.write().compressor_ratio = 16.0;
                //                     },
                //                     label { "16:1" }
                //                 }
                //             }
                //         }
                //     }
                }

                rect { height: "20" }
            }
        }

        // ボタンエリア（ScrollViewの外）
        rect {
            width: "100%",
            height: "80",
            background: "rgb(50, 54, 62)",
            direction: "horizontal",
            main_align: "center",
            cross_align: "center",
            padding: "20",

            FilledButton {
                onpress: move |_| {
                    match settings.read().save() {
                        Ok(_) => save_message.set("設定を保存しました！".to_string()),
                        Err(_) => save_message.set("設定の保存に失敗しました".to_string()),
                    }
                },
                label { "💾 設定を保存" }
            }

            rect { width: "20" }

            Button {
                onpress: move |_| on_navigate_to_recording.call(()),
                label { "🎙️ 録音ページへ" }
            }
        }

        // 保存メッセージ（必要に応じて表示）
        if !save_message.read().is_empty() {
            rect {
                width: "100%",
                height: "auto",
                background: "rgb(50, 54, 62)",
                padding: "10",
                main_align: "center",
                cross_align: "center",

                label {
                    color: if save_message.read().contains("失敗") { "red" } else { "green" },
                    font_size: "14",
                    text_align: "center",
                    "{save_message.read()}"
                }
            }
        }
        }
    }
}
