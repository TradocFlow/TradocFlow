use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;
use tokio::sync::RwLock;
use tradocflow_translation_memory::{
    TradocFlowTranslationMemory, 
    TranslationUnit as ExternalTranslationUnit,
    TranslationMatch as ExternalTranslationMatch,
    Term as ExternalTerm,
    Language, 
    ComprehensiveSearchResult,
};

use crate::models::translation_models::{
    TranslationUnit as LocalTranslationUnit,
    LanguagePair,
    ChunkMetadata,
};

use crate::services::translation_memory_service::{
    TranslationMatch as LocalTranslationMatch,
    TranslationSuggestion as LocalTranslationSuggestion,
    TranslationSource,
    ChunkLinkType,
};

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
    }
    
    /// Search for similar translations
    pub async fn search_similar_translations(
        &self,
        text: &str,
        language_pair: LanguagePair,
    ) -> Result<Vec<LocalTranslationMatch>> {
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
    ) -> Result<Vec<LocalTranslationSuggestion>> {
        let source_lang = Language::English; // Default source language
        let target_lang = self.convert_language(target_language)?;
        
        let result = self.translation_memory
            .comprehensive_search(source_text, source_lang, target_lang)
            .await?;
        
        let mut suggestions = Vec::new();
        
        // Convert translation matches to suggestions
        for unit in result.translation_matches {
            suggestions.push(LocalTranslationSuggestion {
                id: Uuid::new_v4(),
                source_text: unit.source_text.clone(),
                suggested_text: unit.target_text.clone(),
                confidence: unit.quality.unwrap_or(0.8) as f32,
                similarity: 0.9, // Default similarity for now
                context: unit.context,
                source: TranslationSource::Memory,
            });
        }
        
        // Convert terminology matches to suggestions
        for term in result.terminology_matches {
            suggestions.push(LocalTranslationSuggestion {
                id: Uuid::new_v4(),
                source_text: term.source_term.clone(),
                suggested_text: term.target_term.clone(),
                confidence: 0.95, // High confidence for terminology
                similarity: 1.0, // Exact match for terminology
                context: term.context,
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
    pub async fn add_chunks_batch(&self, _chunks: Vec<ChunkMetadata>) -> Result<()> {
        // This functionality would need to be implemented in the new crate
        // For now, we'll return Ok to maintain compatibility
        Ok(())
    }
    
    /// Convert local translation unit to external format
    fn convert_to_external_unit(&self, unit: LocalTranslationUnit) -> Result<ExternalTranslationUnit> {
        use tradocflow_translation_memory::{
            TranslationUnitBuilder,
            TranslationStatus,
            Quality,
        };
        
        let builder = TranslationUnitBuilder::new()
            .source_text(unit.source_text)
            .target_text(unit.target_text)
            .source_language(self.convert_language(&unit.source_language)?)
            .target_language(self.convert_language(&unit.target_language)?)
            .status(TranslationStatus::Final) // Default status
            .quality(Some(Quality::High)); // Default quality
        
        if let Some(context) = unit.context {
            builder.context(context).build()
        } else {
            builder.build()
        }
    }
    
    /// Convert external translation unit to local match format
    fn convert_to_local_match(&self, unit: ExternalTranslationUnit) -> Result<LocalTranslationMatch> {
        Ok(LocalTranslationMatch {
            id: unit.id,
            source_text: unit.source_text,
            target_text: unit.target_text,
            confidence_score: unit.quality.unwrap_or(0.8) as f32,
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
        file_path: &std::path::Path,
        _project_id: Uuid,
    ) -> Result<tradocflow_translation_memory::TerminologyImportResult> {
        self.translation_memory
            .terminology()
            .import_csv_file(file_path)
            .await
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
    }
}