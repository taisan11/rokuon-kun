#![windows_subsystem = "windows"]
use freya::prelude::*;
use dioxus_i18n::prelude::*;
mod record_page;
mod setting_page;
mod effect;
mod i18n;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Recording,
    Settings,
}

fn app() -> Element {
    use_init_i18n(|| i18n::init_i18n());
    let mut current_page = use_signal(|| Page::Recording);

    rsx! {
        match current_page() {
            Page::Recording => rsx! {
                record_page::record_page { 
                    on_navigate_to_settings: move |_| current_page.set(Page::Settings)
                }
            },
            Page::Settings => rsx! {
                setting_page::SettingsPage { 
                    on_navigate_to_recording: move |_| current_page.set(Page::Recording)
                }
            },
        }
    }
}

fn main() {
    launch_with_title(app,"録音くん");
}
