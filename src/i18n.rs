use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

/// Supported languages in the application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "de")] 
    German,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "nl")]
    Dutch,
}

impl Language {
    /// Get the language code as a string
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::German => "de",
            Language::French => "fr",
            Language::Spanish => "es",
            Language::Italian => "it",
            Language::Dutch => "nl",
        }
    }

    /// Get the language name in the target language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::German => "Deutsch",
            Language::French => "Français",
            Language::Spanish => "Español",
            Language::Italian => "Italiano",
            Language::Dutch => "Nederlands",
        }
    }

    /// Parse a language from a string code
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Language::English),
            "de" => Some(Language::German),
            "fr" => Some(Language::French),
            "es" => Some(Language::Spanish),
            "it" => Some(Language::Italian),
            "nl" => Some(Language::Dutch),
            _ => None,
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Self> {
        vec![
            Language::English,
            Language::German,
            Language::French,
            Language::Spanish,
            Language::Italian,
            Language::Dutch,
        ]
    }

    /// Convert to LanguageIdentifier for more advanced language matching
    pub fn to_lang_id(&self) -> LanguageIdentifier {
        self.code().parse().expect("Valid language identifier")
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Thread-local storage for current language
static CURRENT_LANG: OnceLock<std::sync::RwLock<Language>> = OnceLock::new();

/// Initialize the i18n system
pub fn init() {
    let _ = CURRENT_LANG.set(std::sync::RwLock::new(Language::default()));
    
    // Initialize rust-i18n with default locale
    rust_i18n::set_locale("en");
}

/// Set the current language for the thread
pub fn set_language(lang: Language) {
    if let Some(current) = CURRENT_LANG.get() {
        if let Ok(mut guard) = current.write() {
            *guard = lang.clone();
        }
    }
    rust_i18n::set_locale(lang.code());
}

/// Get the current language
pub fn get_language() -> Language {
    if let Some(current) = CURRENT_LANG.get() {
        if let Ok(guard) = current.read() {
            return guard.clone();
        }
    }
    Language::default()
}

/// Translate a key with the current language using rust-i18n
pub fn t(key: &str) -> String {
    // Use the macro from rust-i18n
    rust_i18n::t!(key).to_string()
}

/// Translate a key with arguments using rust-i18n
pub fn t_with_args(key: &str, args: &HashMap<String, String>) -> String {
    // For more complex argument handling, we can use rust-i18n's built-in support
    // For now, use simple placeholder replacement
    let mut result = t(key);
    
    // Simple placeholder replacement for {key} style placeholders
    for (placeholder_key, value) in args {
        let placeholder = format!("{{{}}}", placeholder_key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

/// Helper macro for translations with current language context
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::t($key)
    };
    ($key:expr, $($arg_key:expr => $arg_value:expr),*) => {{
        let mut args = std::collections::HashMap::new();
        $(
            args.insert($arg_key.to_string(), $arg_value.to_string());
        )*
        $crate::i18n::t_with_args($key, &args)
    }};
}

/// Language detection from HTTP Accept-Language header
pub fn detect_language_from_header(accept_language: Option<&str>) -> Language {
    if let Some(header_value) = accept_language {
        // Parse Accept-Language header (simplified)
        for lang_range in header_value.split(',') {
            let lang_code = lang_range
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .split('-')
                .next()
                .unwrap_or("")
                .to_lowercase();
            
            if let Some(lang) = Language::from_code(&lang_code) {
                return lang;
            }
        }
    }
    Language::default()
}

/// Context for templates with i18n support
#[derive(Debug, Clone, Serialize)]
pub struct I18nContext {
    pub language: Language,
    pub language_code: String,
    pub language_name: String,
    pub available_languages: Vec<LanguageInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub display_name: String,
    pub is_current: bool,
}

impl I18nContext {
    pub fn new(current_lang: Language) -> Self {
        let available_languages = Language::all()
            .into_iter()
            .map(|lang| LanguageInfo {
                code: lang.code().to_string(),
                name: lang.code().to_string(),
                display_name: lang.display_name().to_string(),
                is_current: lang == current_lang,
            })
            .collect();

        Self {
            language_code: current_lang.code().to_string(),
            language_name: current_lang.display_name().to_string(),
            language: current_lang,
            available_languages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::German.code(), "de");
        assert_eq!(Language::French.code(), "fr");
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::Italian.code(), "it");
        assert_eq!(Language::Dutch.code(), "nl");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("de"), Some(Language::German));
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_detect_language_from_header() {
        assert_eq!(
            detect_language_from_header(Some("de-DE,de;q=0.9,en;q=0.8")),
            Language::German
        );
        assert_eq!(
            detect_language_from_header(Some("fr-FR,fr;q=0.9")),
            Language::French
        );
        assert_eq!(
            detect_language_from_header(Some("invalid")),
            Language::English
        );
        assert_eq!(detect_language_from_header(None), Language::English);
    }

    #[test]
    fn test_i18n_context() {
        let context = I18nContext::new(Language::German);
        assert_eq!(context.language_code, "de");
        assert_eq!(context.language_name, "Deutsch");
        assert_eq!(context.available_languages.len(), 6);
        
        let current_lang = context.available_languages
            .iter()
            .find(|lang| lang.is_current)
            .unwrap();
        assert_eq!(current_lang.code, "de");
    }
}