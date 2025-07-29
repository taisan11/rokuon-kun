use dioxus_i18n::prelude::*;
use nojson::{DisplayJson, JsonFormatter, JsonParseError, RawJsonValue};
use unic_langid::langid;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Language {
    Japanese,
    English,
}

impl DisplayJson for Language {
    fn fmt(&self, f: &mut JsonFormatter<'_, '_>) -> std::fmt::Result {
        match self {
            Language::Japanese => f.string("ja"),
            Language::English => f.string("en"),
        }
    }
}

impl<'text, 'raw> TryFrom<RawJsonValue<'text, 'raw>> for Language {
    type Error = JsonParseError;

    fn try_from(value: RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let lang_str: String = value.try_into()?;
        match lang_str.as_str() {
            "ja" => Ok(Language::Japanese),
            "en" => Ok(Language::English),
            _ => Err(value.invalid("Invalid language")),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Japanese => write!(f, "ja"),
            Language::English => write!(f, "en"),
        }
    }
}

impl From<Language> for &'static str {
    fn from(lang: Language) -> Self {
        match lang {
            Language::Japanese => "ja",
            Language::English => "en",
        }
    }
}

pub fn init_i18n() -> I18nConfig {
    I18nConfig::new(langid!("ja"))
        .with_locale(Locale::new_static(
            langid!("ja"),
            include_str!("../locales/ja.ftl"),
        ))
        .with_locale(Locale::new_static(
            langid!("en"),
            include_str!("../locales/en.ftl"),
        ))
}
