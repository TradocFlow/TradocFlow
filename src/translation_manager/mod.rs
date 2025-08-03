use crate::{Document, Result, TradocumentError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationProject {
    pub id: Uuid,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub documents: Vec<Uuid>,
    pub translators: HashMap<String, Vec<String>>, // language -> translator_ids
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnit {
    pub id: String,
    pub source_text: String,
    pub translations: HashMap<String, String>, // language -> translated_text
    pub context: Option<String>,
    pub notes: Vec<TranslationNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationNote {
    pub author_id: String,
    pub content: String,
    pub language: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct TranslationManager {
    supported_languages: Vec<String>,
}

impl TranslationManager {
    pub fn new() -> Self {
        Self {
            supported_languages: vec![
                "en".to_string(),
                "de".to_string(),
                "fr".to_string(),
                "es".to_string(),
                "it".to_string(),
                "nl".to_string(),
            ],
        }
    }

    pub fn get_supported_languages(&self) -> &[String] {
        &self.supported_languages
    }

    pub fn is_supported_language(&self, language: &str) -> bool {
        self.supported_languages.contains(&language.to_string())
    }

    pub async fn create_translation_project(
        &self,
        source_language: String,
        target_languages: Vec<String>,
    ) -> Result<TranslationProject> {
        if !self.is_supported_language(&source_language) {
            return Err(TradocumentError::UnsupportedLanguage(source_language));
        }

        for lang in &target_languages {
            if !self.is_supported_language(lang) {
                return Err(TradocumentError::UnsupportedLanguage(lang.clone()));
            }
        }

        Ok(TranslationProject {
            id: Uuid::new_v4(),
            source_language,
            target_languages,
            documents: Vec::new(),
            translators: HashMap::new(),
        })
    }

    pub async fn extract_translation_units(&self, document: &Document) -> Result<Vec<TranslationUnit>> {
        let mut units = Vec::new();
        
        for (language, content) in &document.content {
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.trim().is_empty() {
                    let unit = TranslationUnit {
                        id: format!("{}:{}:{}", document.id, language, i),
                        source_text: line.to_string(),
                        translations: HashMap::new(),
                        context: None,
                        notes: Vec::new(),
                    };
                    units.push(unit);
                }
            }
        }

        Ok(units)
    }

    pub async fn update_translation(
        &self,
        _unit_id: &str,
        language: &str,
        _translation: &str,
    ) -> Result<()> {
        if !self.is_supported_language(language) {
            return Err(TradocumentError::UnsupportedLanguage(language.to_string()));
        }
        
        Ok(())
    }
}