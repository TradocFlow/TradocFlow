use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Language-specific syntax highlighting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSyntaxConfig {
    pub language_code: String,
    pub language_name: String,
    pub text_direction: TextDirection,
    pub font_family: Option<String>,
    pub font_size_multiplier: f32,
    pub line_height_multiplier: f32,
    pub markdown_extensions: Vec<MarkdownExtension>,
    pub special_characters: Vec<SpecialCharacter>,
}

/// Text direction for different languages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

/// Markdown extensions supported by different languages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarkdownExtension {
    Tables,
    TaskLists,
    Strikethrough,
    Footnotes,
    DefinitionLists,
    MathJax,
    Emoji,
}

/// Special characters that need highlighting in specific languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialCharacter {
    pub character: String,
    pub description: String,
    pub highlight_color: String,
    pub requires_attention: bool,
}

/// Syntax highlighting theme for different languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTheme {
    pub theme_name: String,
    pub background_color: String,
    pub text_color: String,
    pub heading_color: String,
    pub link_color: String,
    pub code_background: String,
    pub code_text_color: String,
    pub quote_background: String,
    pub quote_border_color: String,
    pub emphasis_color: String,
    pub strong_color: String,
}

/// Language-specific syntax highlighting service
pub struct LanguageSyntaxService {
    language_configs: HashMap<String, LanguageSyntaxConfig>,
    syntax_themes: HashMap<String, SyntaxTheme>,
}

impl LanguageSyntaxService {
    /// Create a new language syntax service with default configurations
    pub fn new() -> Self {
        let mut service = Self {
            language_configs: HashMap::new(),
            syntax_themes: HashMap::new(),
        };
        
        service.initialize_default_configs();
        service.initialize_default_themes();
        service
    }
    
    /// Initialize default language configurations
    fn initialize_default_configs(&mut self) {
        // English configuration
        self.language_configs.insert("en".to_string(), LanguageSyntaxConfig {
            language_code: "en".to_string(),
            language_name: "English".to_string(),
            text_direction: TextDirection::LeftToRight,
            font_family: Some("Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".to_string()),
            font_size_multiplier: 1.0,
            line_height_multiplier: 1.5,
            markdown_extensions: vec![
                MarkdownExtension::Tables,
                MarkdownExtension::TaskLists,
                MarkdownExtension::Strikethrough,
                MarkdownExtension::Footnotes,
                MarkdownExtension::Emoji,
            ],
            special_characters: vec![],
        });
        
        // German configuration
        self.language_configs.insert("de".to_string(), LanguageSyntaxConfig {
            language_code: "de".to_string(),
            language_name: "Deutsch".to_string(),
            text_direction: TextDirection::LeftToRight,
            font_family: Some("Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".to_string()),
            font_size_multiplier: 1.0,
            line_height_multiplier: 1.5,
            markdown_extensions: vec![
                MarkdownExtension::Tables,
                MarkdownExtension::TaskLists,
                MarkdownExtension::Strikethrough,
                MarkdownExtension::Footnotes,
            ],
            special_characters: vec![
                SpecialCharacter {
                    character: "ß".to_string(),
                    description: "German eszett".to_string(),
                    highlight_color: "#e3f2fd".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "ä".to_string(),
                    description: "German umlaut a".to_string(),
                    highlight_color: "#e3f2fd".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "ö".to_string(),
                    description: "German umlaut o".to_string(),
                    highlight_color: "#e3f2fd".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "ü".to_string(),
                    description: "German umlaut u".to_string(),
                    highlight_color: "#e3f2fd".to_string(),
                    requires_attention: false,
                },
            ],
        });
        
        // French configuration
        self.language_configs.insert("fr".to_string(), LanguageSyntaxConfig {
            language_code: "fr".to_string(),
            language_name: "Français".to_string(),
            text_direction: TextDirection::LeftToRight,
            font_family: Some("Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".to_string()),
            font_size_multiplier: 1.0,
            line_height_multiplier: 1.5,
            markdown_extensions: vec![
                MarkdownExtension::Tables,
                MarkdownExtension::TaskLists,
                MarkdownExtension::Strikethrough,
                MarkdownExtension::Footnotes,
            ],
            special_characters: vec![
                SpecialCharacter {
                    character: "é".to_string(),
                    description: "French acute accent".to_string(),
                    highlight_color: "#fff3e0".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "è".to_string(),
                    description: "French grave accent".to_string(),
                    highlight_color: "#fff3e0".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "ç".to_string(),
                    description: "French cedilla".to_string(),
                    highlight_color: "#fff3e0".to_string(),
                    requires_attention: false,
                },
            ],
        });
        
        // Spanish configuration
        self.language_configs.insert("es".to_string(), LanguageSyntaxConfig {
            language_code: "es".to_string(),
            language_name: "Español".to_string(),
            text_direction: TextDirection::LeftToRight,
            font_family: Some("Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".to_string()),
            font_size_multiplier: 1.0,
            line_height_multiplier: 1.5,
            markdown_extensions: vec![
                MarkdownExtension::Tables,
                MarkdownExtension::TaskLists,
                MarkdownExtension::Strikethrough,
                MarkdownExtension::Footnotes,
            ],
            special_characters: vec![
                SpecialCharacter {
                    character: "ñ".to_string(),
                    description: "Spanish eñe".to_string(),
                    highlight_color: "#f3e5f5".to_string(),
                    requires_attention: false,
                },
                SpecialCharacter {
                    character: "¿".to_string(),
                    description: "Spanish inverted question mark".to_string(),
                    highlight_color: "#f3e5f5".to_string(),
                    requires_attention: true,
                },
                SpecialCharacter {
                    character: "¡".to_string(),
                    description: "Spanish inverted exclamation mark".to_string(),
                    highlight_color: "#f3e5f5".to_string(),
                    requires_attention: true,
                },
            ],
        });
    }
    
    /// Initialize default syntax themes
    fn initialize_default_themes(&mut self) {
        // Light theme
        self.syntax_themes.insert("light".to_string(), SyntaxTheme {
            theme_name: "Light".to_string(),
            background_color: "#ffffff".to_string(),
            text_color: "#1a1a1a".to_string(),
            heading_color: "#2563eb".to_string(),
            link_color: "#0ea5e9".to_string(),
            code_background: "#f8fafc".to_string(),
            code_text_color: "#dc2626".to_string(),
            quote_background: "#f9fafb".to_string(),
            quote_border_color: "#d1d5db".to_string(),
            emphasis_color: "#7c3aed".to_string(),
            strong_color: "#1f2937".to_string(),
        });
        
        // Dark theme
        self.syntax_themes.insert("dark".to_string(), SyntaxTheme {
            theme_name: "Dark".to_string(),
            background_color: "#1a1a1a".to_string(),
            text_color: "#e5e5e5".to_string(),
            heading_color: "#60a5fa".to_string(),
            link_color: "#38bdf8".to_string(),
            code_background: "#262626".to_string(),
            code_text_color: "#f87171".to_string(),
            quote_background: "#262626".to_string(),
            quote_border_color: "#525252".to_string(),
            emphasis_color: "#a78bfa".to_string(),
            strong_color: "#f3f4f6".to_string(),
        });
    }
    
    /// Get language configuration for a specific language
    pub fn get_language_config(&self, language_code: &str) -> Option<&LanguageSyntaxConfig> {
        self.language_configs.get(language_code)
    }
    
    /// Get syntax theme by name
    pub fn get_syntax_theme(&self, theme_name: &str) -> Option<&SyntaxTheme> {
        self.syntax_themes.get(theme_name)
    }
    
    /// Add or update language configuration
    pub fn set_language_config(&mut self, config: LanguageSyntaxConfig) {
        self.language_configs.insert(config.language_code.clone(), config);
    }
    
    /// Add or update syntax theme
    pub fn set_syntax_theme(&mut self, theme: SyntaxTheme) {
        self.syntax_themes.insert(theme.theme_name.clone(), theme);
    }
    
    /// Get all supported languages
    pub fn get_supported_languages(&self) -> Vec<&str> {
        self.language_configs.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get all available themes
    pub fn get_available_themes(&self) -> Vec<&str> {
        self.syntax_themes.keys().map(|s| s.as_str()).collect()
    }
    
    /// Generate CSS for language-specific styling
    pub fn generate_language_css(&self, language_code: &str, theme_name: &str) -> String {
        let config = self.get_language_config(language_code);
        let theme = self.get_syntax_theme(theme_name);
        
        if let (Some(config), Some(theme)) = (config, theme) {
            let direction = match config.text_direction {
                TextDirection::LeftToRight => "ltr",
                TextDirection::RightToLeft => "rtl",
                TextDirection::TopToBottom => "ttb",
            };
            
            let font_family = config.font_family.as_deref().unwrap_or("monospace");
            
            format!(
                r#"
                .editor-{} {{
                    direction: {};
                    font-family: {};
                    font-size: calc(1rem * {});
                    line-height: {};
                    background-color: {};
                    color: {};
                }}
                
                .editor-{} h1, .editor-{} h2, .editor-{} h3, 
                .editor-{} h4, .editor-{} h5, .editor-{} h6 {{
                    color: {};
                }}
                
                .editor-{} a {{
                    color: {};
                }}
                
                .editor-{} code {{
                    background-color: {};
                    color: {};
                    padding: 0.125rem 0.25rem;
                    border-radius: 0.25rem;
                }}
                
                .editor-{} blockquote {{
                    background-color: {};
                    border-left: 4px solid {};
                    padding: 0.5rem 1rem;
                    margin: 0.5rem 0;
                }}
                
                .editor-{} em {{
                    color: {};
                }}
                
                .editor-{} strong {{
                    color: {};
                    font-weight: 600;
                }}
                "#,
                language_code, direction, font_family, 
                config.font_size_multiplier, config.line_height_multiplier,
                theme.background_color, theme.text_color,
                language_code, language_code, language_code,
                language_code, language_code, language_code,
                theme.heading_color,
                language_code, theme.link_color,
                language_code, theme.code_background, theme.code_text_color,
                language_code, theme.quote_background, theme.quote_border_color,
                language_code, theme.emphasis_color,
                language_code, theme.strong_color
            )
        } else {
            String::new()
        }
    }
    
    /// Check if a character is special for a given language
    pub fn is_special_character(&self, language_code: &str, character: &str) -> Option<&SpecialCharacter> {
        if let Some(config) = self.get_language_config(language_code) {
            config.special_characters.iter()
                .find(|sc| sc.character == character)
        } else {
            None
        }
    }
    
    /// Get markdown extensions supported by a language
    pub fn get_markdown_extensions(&self, language_code: &str) -> Vec<MarkdownExtension> {
        self.get_language_config(language_code)
            .map(|config| config.markdown_extensions.clone())
            .unwrap_or_default()
    }
}

impl Default for LanguageSyntaxService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_syntax_service_creation() {
        let service = LanguageSyntaxService::new();
        
        assert!(service.get_language_config("en").is_some());
        assert!(service.get_language_config("de").is_some());
        assert!(service.get_language_config("fr").is_some());
        assert!(service.get_language_config("es").is_some());
        
        assert!(service.get_syntax_theme("light").is_some());
        assert!(service.get_syntax_theme("dark").is_some());
    }
    
    #[test]
    fn test_language_config_retrieval() {
        let service = LanguageSyntaxService::new();
        
        let en_config = service.get_language_config("en").unwrap();
        assert_eq!(en_config.language_code, "en");
        assert_eq!(en_config.text_direction, TextDirection::LeftToRight);
        assert_eq!(en_config.font_size_multiplier, 1.0);
        
        let de_config = service.get_language_config("de").unwrap();
        assert_eq!(de_config.language_code, "de");
        assert!(!de_config.special_characters.is_empty());
    }
    
    #[test]
    fn test_special_character_detection() {
        let service = LanguageSyntaxService::new();
        
        assert!(service.is_special_character("de", "ß").is_some());
        assert!(service.is_special_character("fr", "é").is_some());
        assert!(service.is_special_character("es", "ñ").is_some());
        assert!(service.is_special_character("en", "ß").is_none());
    }
    
    #[test]
    fn test_css_generation() {
        let service = LanguageSyntaxService::new();
        
        let css = service.generate_language_css("en", "light");
        assert!(css.contains(".editor-en"));
        assert!(css.contains("direction: ltr"));
        assert!(css.contains("background-color: #ffffff"));
        
        let css_dark = service.generate_language_css("de", "dark");
        assert!(css_dark.contains(".editor-de"));
        assert!(css_dark.contains("background-color: #1a1a1a"));
    }
    
    #[test]
    fn test_markdown_extensions() {
        let service = LanguageSyntaxService::new();
        
        let en_extensions = service.get_markdown_extensions("en");
        assert!(en_extensions.contains(&MarkdownExtension::Tables));
        assert!(en_extensions.contains(&MarkdownExtension::Emoji));
        
        let de_extensions = service.get_markdown_extensions("de");
        assert!(de_extensions.contains(&MarkdownExtension::Tables));
        assert!(!de_extensions.contains(&MarkdownExtension::Emoji));
    }
}