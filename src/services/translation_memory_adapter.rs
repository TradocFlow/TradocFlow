use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tradocflow_translation_memory::{
    TradocFlowTranslationMemory, 
    TranslationUnit as ExternalTranslationUnit,
    Term as ExternalTerm,
    Language,
};

use crate::models::{
    document::TranslationUnit as LocalTranslationUnit,
    translation_models::LanguagePair,
};

// Stub types for compatibility
#[derive(Debug, Clone)]
pub struct TranslationMatch {
    pub id: Uuid,
    pub source_text: String,
    pub target_text: String,
    pub confidence_score: f32,
    pub similarity_score: f32,
    pub context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranslationSuggestion {
    pub id: Uuid,
    pub source_text: String,
    pub suggested_text: String,
    pub confidence: f32,
    pub similarity: f32,
    pub context: Option<String>,
    pub source: TranslationSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TranslationSource {
    Memory,
    Terminology,
    MachineTranslation,
    Manual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkLinkType {
    LinkedPhrase,
    Unlinked,
    Merged,
}

/// Adapter service that bridges between the old TranslationMemoryService API
/// and the new tradocflow-translation-memory crate
#[derive(Clone)]
pub struct TranslationMemoryAdapter {
    translation_memory: Arc<TradocFlowTranslationMemory>,
    project_path: PathBuf,
}

impl TranslationMemoryAdapter {
    /// Create a new adapter instance
    pub async fn new(project_path: PathBuf) -> Result<Self> {
        let tm_path = project_path.join("translation_memory");
        std::fs::create_dir_all(&tm_path)?;
        
        let db_path = tm_path.join("index.db");
        let translation_memory = Arc::new(
            TradocFlowTranslationMemory::new(db_path.to_str().unwrap()).await?
        );
        
        // Initialize the database schema
        translation_memory.initialize().await?;
        
        Ok(Self {
            translation_memory,
            project_path,
        })
    }
    
    /// Create translation memory for a project
    pub async fn create_translation_memory(&self, _project_id: Uuid) -> Result<()> {
        // The new crate handles this automatically during initialization
        Ok(())
    }
    
    /// Add a single translation unit
    pub async fn add_translation_unit(&self, unit: LocalTranslationUnit) -> Result<()> {
        let external_unit = self.convert_to_external_unit(unit)?;
        self.translation_memory
            .translation_memory()
            .add_translation_unit(external_unit)
            .await
            .map_err(|e| anyhow::anyhow!("Translation memory error: {}", e))
    }
    
    /// Add multiple translation units in batch
    pub async fn add_translation_units_batch(&self, units: Vec<LocalTranslationUnit>) -> Result<()> {
        let external_units: Result<Vec<_>> = units
            .into_iter()
            .map(|unit| self.convert_to_external_unit(unit))
            .collect();
        
        self.translation_memory
            .translation_memory()
            .add_translation_units_batch(external_units?)
            .await
            .map(|_| ()) // Convert usize result to ()
            .map_err(|e| anyhow::anyhow!("Translation memory error: {}", e))
    }
    
    /// Search for similar translations
    pub async fn search_similar_translations(
        &self,
        text: &str,
        language_pair: LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        let source_lang = self.convert_language(&language_pair.source)?;
        let target_lang = self.convert_language(&language_pair.target)?;
        
        let units = self.translation_memory
            .translation_memory()
            .search(text, source_lang, target_lang, 0.7)
            .await?;
        
        let matches = units
            .into_iter()
            .map(|unit| self.convert_to_local_match(unit))
            .collect::<Result<Vec<_>>>()?;
        
        Ok(matches)
    }
    
    /// Get translation suggestions
    pub async fn get_translation_suggestions(
        &self,
        source_text: &str,
        target_language: &str,
    ) -> Result<Vec<TranslationSuggestion>> {
        let source_lang = Language::English; // Default source language
        let target_lang = self.convert_language(target_language)?;
        
        let result = self.translation_memory
            .comprehensive_search(source_text, source_lang, target_lang)
            .await?;
        
        let mut suggestions = Vec::new();
        
        // Convert translation matches to suggestions
        for unit in result.translation_matches {
            suggestions.push(TranslationSuggestion {
                id: Uuid::new_v4(),
                source_text: unit.source_text.clone(),
                suggested_text: unit.target_text.clone(),
                confidence: 0.8, // Default confidence
                similarity: 0.9, // Default similarity for now
                context: unit.context,
                source: TranslationSource::Memory,
            });
        }
        
        // Convert terminology matches to suggestions
        for term in result.terminology_matches {
            suggestions.push(TranslationSuggestion {
                id: Uuid::new_v4(),
                source_text: term.term.clone(),
                suggested_text: term.definition.clone().unwrap_or_default(),
                confidence: 0.95, // High confidence for terminology
                similarity: 1.0, // Exact match for terminology
                context: None,
                source: TranslationSource::Terminology,
            });
        }
        
        Ok(suggestions)
    }
    
    /// Update chunk linking (legacy compatibility)
    pub async fn update_chunk_linking(
        &self,
        _chunk_ids: Vec<Uuid>,
        _link_type: ChunkLinkType,
    ) -> Result<()> {
        // This functionality would need to be implemented in the new crate
        // For now, we'll return Ok to maintain compatibility
        Ok(())
    }
    
    /// Add multiple chunks (legacy compatibility)
    /// TODO: Implement when ChunkMetadata is defined
    pub async fn add_chunks_batch(&self, _chunks: Vec<String>) -> Result<()> {
        // This functionality would need to be implemented in the new crate
        // For now, we'll return Ok to maintain compatibility
        Ok(())
    }
    
    /// Convert local translation unit to external format
    fn convert_to_external_unit(&self, unit: LocalTranslationUnit) -> Result<ExternalTranslationUnit> {
        use tradocflow_translation_memory::{
            TranslationUnitBuilder,
        };
        
        // For now, we'll use a simplified conversion
        // The LocalTranslationUnit has translations HashMap, so we'll take the first available translation
        let target_text = unit.translations.values().next()
            .map(|v| v.text.clone())
            .unwrap_or_default();
        
        let source_lang = self.convert_language(&unit.source_language)?;
        let source_lang_code = self.language_to_string(source_lang);
        let builder = TranslationUnitBuilder::new()
            .source_text(unit.source_text)
            .target_text(target_text)
            .source_language(&source_lang_code)
            .target_language("en"); // Default target language for now
        
        if let Some(context) = unit.context {
            Ok(builder.context(context).build()?)
        } else {
            Ok(builder.build()?)
        }
    }
    
    /// Convert external translation unit to local match format
    fn convert_to_local_match(&self, unit: ExternalTranslationUnit) -> Result<TranslationMatch> {
        Ok(TranslationMatch {
            id: unit.id,
            source_text: unit.source_text,
            target_text: unit.target_text,
            confidence_score: 0.8, // Default confidence
            similarity_score: 0.9, // Default similarity
            context: unit.context,
        })
    }
    
    /// Convert language string to external Language enum
    fn convert_language(&self, lang: &str) -> Result<Language> {
        match lang.to_lowercase().as_str() {
            "en" | "english" => Ok(Language::English),
            "es" | "spanish" => Ok(Language::Spanish),
            "fr" | "french" => Ok(Language::French),
            "de" | "german" => Ok(Language::German),
            "it" | "italian" => Ok(Language::Italian),
            "pt" | "portuguese" => Ok(Language::Portuguese),
            "zh" | "chinese" => Ok(Language::Chinese),
            "ja" | "japanese" => Ok(Language::Japanese),
            "ko" | "korean" => Ok(Language::Korean),
            "ru" | "russian" => Ok(Language::Russian),
            "ar" | "arabic" => Ok(Language::Arabic),
            "hi" | "hindi" => Ok(Language::Hindi),
            "nl" | "dutch" => Ok(Language::Dutch),
            _ => Ok(Language::English), // Default fallback
        }
    }
    
    /// Convert Language enum to string
    fn language_to_string(&self, lang: Language) -> String {
        lang.code().to_string()
    }
    
    /// Get cache statistics (compatibility method)
    pub async fn get_cache_stats(&self) -> (usize, usize, Option<chrono::DateTime<chrono::Utc>>) {
        // Return default values for compatibility
        (0, 0, Some(chrono::Utc::now()))
    }
}

/// Terminology service adapter
#[derive(Clone)]
pub struct TerminologyServiceAdapter {
    translation_memory: Arc<TradocFlowTranslationMemory>,
}

impl TerminologyServiceAdapter {
    /// Create a new terminology service adapter
    pub async fn new(project_path: PathBuf) -> Result<Self> {
        let tm_path = project_path.join("translation_memory");
        let db_path = tm_path.join("index.db");
        let translation_memory = Arc::new(
            TradocFlowTranslationMemory::new(db_path.to_str().unwrap()).await?
        );
        
        Ok(Self { translation_memory })
    }
    
    /// Import terminology from CSV
    pub async fn import_terminology_csv(
        &self,
        _file_path: &std::path::Path,
        _project_id: Uuid,
    ) -> Result<tradocflow_translation_memory::TerminologyImportResult> {
        // Stub implementation for now
        Ok(tradocflow_translation_memory::TerminologyImportResult {
            successful_imports: Vec::new(),
            failed_imports: Vec::new(),
            duplicate_terms: Vec::new(),
            total_processed: 0,
        })
    }
    
    /// Get non-translatable terms
    pub async fn get_non_translatable_terms(&self, _project_id: Uuid) -> Result<Vec<ExternalTerm>> {
        // This would need to be implemented based on the new API
        Ok(Vec::new())
    }
    
    /// Search terms
    pub async fn search_terms(
        &self,
        query: &str,
        source_lang: Language,
        target_lang: Language,
    ) -> Result<Vec<ExternalTerm>> {
        self.translation_memory
            .terminology()
            .search_terms(query, source_lang, target_lang)
            .await
            .map_err(|e| anyhow::anyhow!("Translation memory error: {}", e))
    }
}