use freya::prelude::*;
use nojson::{Json, json, DisplayJson, JsonFormatter, JsonParseError, RawJsonValue};
use std::fs;
use std::path::Path;

#[derive(Clone, PartialEq)]
pub struct AppSettings {
    pub audio_format: AudioFormat,
    pub sample_rate: u32,
    pub bit_depth: u16,
}

#[derive(Clone, PartialEq)]
pub enum AudioFormat {
    Wave,
    Pcm,
}

//設定項目の定義...?
impl DisplayJson for AppSettings {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> std::fmt::Result {
        f.object(|f| {
            f.member("audio_format", match self.audio_format {
                AudioFormat::Wave => "WAVE",
                AudioFormat::Pcm => "PCM",
            })?;
            f.member("sample_rate", self.sample_rate)?;
            f.member("bit_depth", self.bit_depth)
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
            _ => return Err(value.invalid("Invalid audio format")),
        };
        
        let sample_rate = value.to_member("sample_rate")?.required()?.try_into()?;
        let bit_depth = value.to_member("bit_depth")?.required()?.try_into()?;
        
        Ok(AppSettings {
            audio_format,
            sample_rate,
            bit_depth,
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
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        if Path::new("settings.json").exists() {
            match fs::read_to_string("settings.json") {
                Ok(content) => {
                    match content.parse::<Json<AppSettings>>() {
                        Ok(settings) => settings.0,
                        Err(_) => Self::default(),
                    }
                }
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
        }).to_string();
        
        fs::write("settings.json", json_content)?;
        Ok(())
    }
}

#[component]
pub fn SettingsPage(on_navigate_to_recording: EventHandler<()>) -> Element {
    let mut settings = use_signal(|| AppSettings::load());
    let mut save_message = use_signal(|| String::new());

    rsx! {
        rect {
            width: "100%",
            height: "100%",
            background: "rgb(40, 44, 52)",
            direction: "vertical",

            // 設定ページの内容
            rect {
                width: "100%",
                height: "calc(100% - 60)",
                padding: "20",
                direction: "vertical",

                label {
                    color: "white",
                    font_size: "28",
                    "設定ページ"
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
                        "音声フォーマット設定"
                    }

                    rect { height: "15" }

                    rect {
                        direction: "horizontal",
                        cross_align: "center",

                        label {
                            color: "white",
                            font_size: "16",
                            width: "120",
                            "保存形式: "
                        }

                        Dropdown {
                            value: match settings.read().audio_format {
                                AudioFormat::Wave => "WAVE",
                                AudioFormat::Pcm => "PCM",
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

                    rect { height: "20" }

                    // 保存ボタン
                    rect {
                        direction: "horizontal",
                        main_align: "center",

                        FilledButton {
                            onpress: move |_| {
                                match settings.read().save() {
                                    Ok(_) => save_message.set("設定を保存しました！".to_string()),
                                    Err(_) => save_message.set("設定の保存に失敗しました".to_string()),
                                }
                            },
                            label { "💾 設定を保存" }
                        }
                    }

                    if !save_message.read().is_empty() {
                        rect { height: "10" }
                        label {
                            color: if save_message.read().contains("失敗") { "red" } else { "green" },
                            font_size: "14",
                            text_align: "center",
                            "{save_message.read()}"
                        }
                    }
                }
            }
            
            // ページ下部のリンクボタン
            rect {
                width: "100%",
                height: "60",
                direction: "horizontal",
                main_align: "center",
                cross_align: "center",
                background: "rgb(50, 54, 62)",
                padding: "10",
                
                Button {
                    onpress: move |_| on_navigate_to_recording.call(()),
                    label { "🎙️ 録音ページへ" }
                }
            }
        }
    }
}
