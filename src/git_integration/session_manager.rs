//! Translation Session Manager
//! 
//! Manages active translation sessions with Git branch coordination and auto-save functionality.

use super::{WorkSession, GitWorkflowManager, ChapterData};
use crate::{Result, TradocumentError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, Duration};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Additional session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub total_time_active: Duration,
    pub commits_made: u32,
    pub units_modified: u32,
    pub quality_scores: Vec<f32>,
    pub editor_preferences: EditorPreferences,
}

/// Editor preferences for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    pub auto_save_enabled: bool,
    pub auto_save_interval: Duration,
    pub word_wrap: bool,
    pub spell_check: bool,
    pub show_word_count: bool,
}

/// Session manager coordinates active translation sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    git_manager: Arc<GitWorkflowManager>,
    active_sessions: Arc<RwLock<HashMap<Uuid, Arc<Mutex<ActiveSession>>>>>,
    auto_save_enabled: bool,
    auto_save_interval: Duration,
}

/// Active session with enhanced state tracking
#[derive(Debug)]
pub struct ActiveSession {
    pub session: WorkSession,
    pub current_content: String,
    pub last_save_content: String,
    pub has_unsaved_changes: bool,
    pub needs_auto_save: bool,
    pub toml_data: Option<ChapterData>,
    pub markdown_generated_at: Option<DateTime<Utc>>,
    pub last_activity: DateTime<Utc>,
    pub save_count: u32,
    pub word_count: usize,
    pub character_count: usize,
    pub auto_save_failures: u32,
    pub session_metadata: SessionMetadata,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(
        git_manager: Arc<GitWorkflowManager>,
        auto_save_interval_seconds: u64,
    ) -> Self {
        Self {
            git_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            auto_save_enabled: true,
            auto_save_interval: Duration::from_secs(auto_save_interval_seconds),
        }
    }

    /// Clone for shared access - shares the same active sessions and git manager
    pub fn clone_for_shared_access(&self) -> Self {
        Self {
            git_manager: Arc::clone(&self.git_manager),
            active_sessions: Arc::clone(&self.active_sessions),
            auto_save_enabled: self.auto_save_enabled,
            auto_save_interval: self.auto_save_interval,
        }
    }

    /// Start a new translation session
    pub async fn start_session(
        &self,
        chapter: &str,
        language: &str,
    ) -> Result<Uuid> {
        // Create Git session
        let work_session = self.git_manager
            .start_translation_session(chapter, language)
            .await?;

        // Load existing TOML data
        let toml_data = self.load_chapter_toml(chapter).await?;
        
        // Generate initial markdown content
        let markdown_content = self.generate_markdown_from_toml(&toml_data, language).await?;
        
        let active_session = ActiveSession {
            session: work_session.clone(),
            current_content: markdown_content.clone(),
            last_save_content: markdown_content,
            has_unsaved_changes: false,
            needs_auto_save: false,
            toml_data: Some(toml_data),
            markdown_generated_at: Some(Utc::now()),
            last_activity: Utc::now(),
            save_count: 0,
            word_count: 0,
            character_count: 0,
            auto_save_failures: 0,
            session_metadata: SessionMetadata::default(),
        };

        // Store active session
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(work_session.id, Arc::new(Mutex::new(active_session)));

        // Start auto-save task for this session
        if self.auto_save_enabled {
            self.start_auto_save_task(work_session.id).await;
        }

        Ok(work_session.id)
    }

    /// Update content in an active session
    pub async fn update_content(
        &self,
        session_id: Uuid,
        content: &str,
    ) -> Result<()> {
        let sessions = self.active_sessions.read().await;
        
        if let Some(active_session_arc) = sessions.get(&session_id) {
            let mut active_session = active_session_arc.lock().await;
            
            // Update content and mark as having unsaved changes
            active_session.current_content = content.to_string();
            active_session.has_unsaved_changes = 
                active_session.current_content != active_session.last_save_content;
                
            // Update session timestamp
            // Note: In a real implementation, we'd update last_activity_at
        } else {
            return Err(TradocumentError::ApiError(
                format!("Session {session_id} not found")
            ));
        }

        Ok(())
    }

    /// Get current content from a session
    pub async fn get_content(&self, session_id: Uuid) -> Result<String> {
        let sessions = self.active_sessions.read().await;
        
        if let Some(active_session_arc) = sessions.get(&session_id) {
            let active_session = active_session_arc.lock().await;
            Ok(active_session.current_content.clone())
        } else {
            Err(TradocumentError::ApiError(
                format!("Session {session_id} not found")
            ))
        }
    }

    /// Manually save a session
    pub async fn save_session(&self, session_id: Uuid) -> Result<()> {
        let sessions = self.active_sessions.read().await;
        
        if let Some(active_session_arc) = sessions.get(&session_id) {
            let mut active_session = active_session_arc.lock().await;
            
            if active_session.has_unsaved_changes {
                // Save to Git
                self.git_manager.auto_save_changes(
                    &active_session.session,
                    &active_session.current_content,
                ).await?;

                // Update TOML data from markdown
                let content = active_session.current_content.clone();
                self.sync_toml_from_markdown(
                    &mut active_session,
                    &content,
                ).await?;

                // Mark as saved
                active_session.last_save_content = active_session.current_content.clone();
                active_session.has_unsaved_changes = false;
            }
        } else {
            return Err(TradocumentError::ApiError(
                format!("Session {session_id} not found")
            ));
        }

        Ok(())
    }

    /// Submit session for review
    pub async fn submit_for_review(
        &self,
        session_id: Uuid,
        description: &str,
    ) -> Result<()> {
        // Save any pending changes first
        self.save_session(session_id).await?;

        let sessions = self.active_sessions.read().await;
        
        if let Some(active_session_arc) = sessions.get(&session_id) {
            let active_session = active_session_arc.lock().await;
            
            // Submit to Git workflow
            let _review_request = self.git_manager.submit_for_review(
                &active_session.session,
                description,
            ).await?;

            // Session becomes read-only after submission
            // In a full implementation, we'd update session status
        } else {
            return Err(TradocumentError::ApiError(
                format!("Session {session_id} not found")
            ));
        }

        Ok(())
    }

    /// End a translation session
    pub async fn end_session(&self, session_id: Uuid) -> Result<()> {
        // Save any pending changes
        if let Err(_) = self.save_session(session_id).await {
            // Log warning but don't fail - session might already be closed
        }

        // Remove from active sessions
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(&session_id);

        Ok(())
    }

    /// List all active sessions
    pub async fn list_active_sessions(&self) -> Result<Vec<WorkSession>> {
        let sessions = self.active_sessions.read().await;
        let mut result = Vec::new();

        for active_session_arc in sessions.values() {
            let active_session = active_session_arc.lock().await;
            result.push(active_session.session.clone());
        }

        Ok(result)
    }

    /// Get session info including unsaved changes status
    pub async fn get_session_info(&self, session_id: Uuid) -> Result<SessionInfo> {
        let sessions = self.active_sessions.read().await;
        
        if let Some(active_session_arc) = sessions.get(&session_id) {
            let active_session = active_session_arc.lock().await;
            
            Ok(SessionInfo {
                session: active_session.session.clone(),
                has_unsaved_changes: active_session.has_unsaved_changes,
                content_length: active_session.current_content.len(),
                last_generated: active_session.markdown_generated_at,
                toml_loaded: active_session.toml_data.is_some(),
            })
        } else {
            Err(TradocumentError::ApiError(
                format!("Session {session_id} not found")
            ))
        }
    }

    // Private helper methods

    async fn load_chapter_toml(&self, chapter: &str) -> Result<ChapterData> {
        let toml_path = format!("content/chapters/{chapter}.toml");
        
        if Path::new(&toml_path).exists() {
            let toml_content = std::fs::read_to_string(&toml_path)?;
            let chapter_data: ChapterData = toml::from_str(&toml_content)
                .map_err(TradocumentError::Toml)?;
            Ok(chapter_data)
        } else {
            // Create new chapter data structure
            Ok(self.create_new_chapter_data(chapter).await?)
        }
    }

    async fn create_new_chapter_data(&self, chapter: &str) -> Result<ChapterData> {
        use std::collections::HashMap;

        let chapter_data = ChapterData {
            chapter: super::models::ChapterMetadata {
                number: 1, // Would be determined by chapter sequence
                slug: chapter.to_string(),
                status: super::models::ChapterStatus::Draft,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                git_branch: None,
                last_git_commit: None,
                title: HashMap::new(),
                metadata: super::models::ChapterMetadataExtra {
                    word_count: HashMap::new(),
                    difficulty: super::models::DifficultyLevel::Beginner,
                    estimated_translation_time: HashMap::new(),
                    requires_screenshots: false,
                    screenshot_count: 0,
                    last_reviewed: HashMap::new(),
                },
            },
            units: Vec::new(),
            todos: Vec::new(),
            comments: Vec::new(),
        };

        Ok(chapter_data)
    }

    async fn generate_markdown_from_toml(
        &self,
        toml_data: &ChapterData,
        language: &str,
    ) -> Result<String> {
        let mut markdown = String::new();

        // Add chapter title
        if let Some(title) = toml_data.chapter.title.get(language) {
            markdown.push_str(&format!("# {title}\n\n"));
        } else if let Some(en_title) = toml_data.chapter.title.get("en") {
            markdown.push_str(&format!("# {en_title} [NEEDS TRANSLATION]\n\n"));
        }

        // Add translation units
        for unit in &toml_data.units {
            if let Some(translation) = unit.translations.get(language) {
                // Use existing translation
                markdown.push_str(&translation.text);
                markdown.push_str("\n\n");
            } else {
                // Show source text with translation placeholder
                markdown.push_str(&format!(
                    "{} [TRANSLATE FROM {}]\n\n",
                    unit.source_text,
                    unit.source_language.to_uppercase()
                ));
            }

            // Add inline todos and comments for this unit
            for todo in &unit.todos {
                if matches!(todo.context, super::models::TodoContext::Translation { ref language, .. } if language == language) {
                    markdown.push_str(&format!(
                        "<!-- TODO: {} (Priority: {:?}) -->\n",
                        todo.title,
                        todo.priority
                    ));
                }
            }
        }

        Ok(markdown)
    }

    async fn sync_toml_from_markdown(
        &self,
        active_session: &mut ActiveSession,
        markdown_content: &str,
    ) -> Result<()> {
        // Parse markdown and update TOML data
        // This is a simplified implementation - in production would use a proper markdown parser
        
        if let Some(ref mut toml_data) = active_session.toml_data {
            // Update translation units from markdown content
            let lines: Vec<&str> = markdown_content.lines().collect();
            let mut current_unit_index = 0;

            for line in lines {
                if !line.trim().is_empty() && !line.starts_with('#') && !line.starts_with("<!--")
                    && current_unit_index < toml_data.units.len() {
                        let unit = &mut toml_data.units[current_unit_index];
                        
                        // Update or create translation for this language
                        let language = &active_session.session.language;
                        
                        if let Some(translation) = unit.translations.get_mut(language) {
                            translation.text = line.to_string();
                            translation.updated_at = Utc::now();
                            translation.revision_count += 1;
                        } else {
                            // Create new translation
                            let translation = super::models::TranslationVersion {
                                text: line.to_string(),
                                translator: active_session.session.user_id.clone(),
                                status: super::models::TranslationUnitStatus::InProgress,
                                quality_score: None,
                                created_at: Utc::now(),
                                updated_at: Utc::now(),
                                reviewed_at: None,
                                reviewer: None,
                                revision_count: 1,
                                metadata: super::models::TranslationMetadata {
                                    terminology_verified: false,
                                    style_guide_compliant: false,
                                    review_notes: None,
                                    translation_method: super::models::TranslationMethod::Human,
                                    confidence_score: 0.8,
                                },
                            };
                            
                            unit.translations.insert(language.clone(), translation);
                        }

                        current_unit_index += 1;
                    }
            }

            // Save updated TOML data
            self.save_toml_data(&active_session.session.chapter, toml_data).await?;
        }

        Ok(())
    }

    async fn save_toml_data(&self, chapter: &str, toml_data: &ChapterData) -> Result<()> {
        let toml_path = format!("content/chapters/{chapter}.toml");
        
        // Ensure directory exists
        if let Some(parent) = Path::new(&toml_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_content = toml::to_string_pretty(toml_data)
            .map_err(|e| TradocumentError::ApiError(format!("TOML serialization error: {e}")))?;
            
        std::fs::write(&toml_path, toml_content)?;
        
        Ok(())
    }

    async fn start_auto_save_task(&self, session_id: Uuid) {
        let sessions_clone = self.active_sessions.clone();
        let interval_duration = self.auto_save_interval;

        // Store the session ID for later auto-save processing
        // The actual auto-save will be handled by a periodic background task
        // or on-demand through the API to avoid thread safety issues
        tokio::spawn(async move {
            let mut interval = interval(interval_duration);
            
            loop {
                interval.tick().await;
                
                // Check if session still exists and mark for auto-save
                let sessions = sessions_clone.read().await;
                if let Some(active_session_arc) = sessions.get(&session_id) {
                    let mut active_session = active_session_arc.lock().await;
                    
                    if active_session.has_unsaved_changes {
                        // Mark session as needing auto-save
                        // The actual auto-save will be handled by external calls to auto_save_session()
                        active_session.needs_auto_save = true;
                        println!("Session {session_id} marked for auto-save");
                    }
                } else {
                    // Session no longer exists, stop auto-save task
                    break;
                }
            }
        });
    }
}

/// Enhanced session information with metrics and productivity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSessionInfo {
    pub session: WorkSession,
    pub has_unsaved_changes: bool,
    pub content_length: usize,
    pub word_count: usize,
    pub character_count: usize,
    pub last_generated: Option<DateTime<Utc>>,
    pub toml_loaded: bool,
    pub save_count: u32,
    pub auto_save_failures: u32,
    pub time_since_last_activity: Duration,
    pub session_duration: Duration,
    pub productivity_metrics: ProductivityMetrics,
    pub editor_preferences: EditorPreferences,
}

/// Legacy session info for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session: WorkSession,
    pub has_unsaved_changes: bool,
    pub content_length: usize,
    pub last_generated: Option<DateTime<Utc>>,
    pub toml_loaded: bool,
}

/// Session manager configuration
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    pub auto_save_enabled: bool,
    pub auto_save_interval: Duration,
    pub max_concurrent_sessions: usize,
    pub session_timeout: Duration,
    pub recovery_enabled: bool,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval: Duration::from_secs(300), // 5 minutes
            max_concurrent_sessions: 10,
            session_timeout: Duration::from_secs(3600), // 1 hour
            recovery_enabled: true,
        }
    }
}

/// Result of a session content update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdateResult {
    pub word_count_change: i32,
    pub character_count_change: i32,
    pub content_similarity: f32,
    pub has_unsaved_changes: bool,
    pub units_potentially_modified: u32,
    pub time_since_last_save: Duration,
}

/// Result of a session save operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSaveResult {
    pub success: bool,
    pub save_duration: Duration,
    pub word_count: usize,
    pub character_count: usize,
    pub commit_count: u32,
    pub errors: Vec<String>,
}

/// Productivity metrics for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityMetrics {
    pub words_per_minute: f64,
    pub commits_per_hour: f64,
    pub units_modified: u32,
    pub average_quality_score: Option<f32>,
}

/// Session recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecoveryInfo {
    pub session_id: Uuid,
    pub branch_name: String,
    pub last_save_content: String,
    pub current_content: String,
    pub has_unsaved_changes: bool,
    pub last_activity: DateTime<Utc>,
    pub save_count: u32,
    pub chapter: String,
    pub language: String,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval: Duration::from_secs(300),
            word_wrap: true,
            spell_check: true,
            show_word_count: true,
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            total_time_active: Duration::from_secs(0),
            commits_made: 0,
            units_modified: 0,
            quality_scores: Vec::new(),
            editor_preferences: EditorPreferences::default(),
        }
    }
}

impl From<EnhancedSessionInfo> for SessionInfo {
    fn from(enhanced: EnhancedSessionInfo) -> Self {
        Self {
            session: enhanced.session,
            has_unsaved_changes: enhanced.has_unsaved_changes,
            content_length: enhanced.content_length,
            last_generated: enhanced.last_generated,
            toml_loaded: enhanced.toml_loaded,
        }
    }
}