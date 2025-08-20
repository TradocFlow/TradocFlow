use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock as TokioRwLock, Mutex as TokioMutex};
use tokio::time::sleep;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use super::markdown_text_processor::{MarkdownTextProcessor, TextProcessorError};
use super::markdown_processor::{MarkdownProcessor, TextRange, ValidationError, ProcessingStatistics};

/// Document modification event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChange {
    pub id: Uuid,
    pub timestamp: u64,
    pub change_type: ChangeType,
    pub position: TextRange,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub author: Option<String>,
    pub checksum: String,
    pub metadata: HashMap<String, String>,
}

/// Types of document changes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Delete,
    Replace,
    Format,
    MetadataUpdate,
    StateSnapshot,
}

/// Document version snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub id: Uuid,
    pub version_number: u64,
    pub timestamp: u64,
    pub content: String,
    pub changes: Vec<DocumentChange>,
    pub checksum: String,
    pub author: Option<String>,
    pub description: String,
    pub statistics: Option<ProcessingStatistics>,
    pub validation_errors: Vec<ValidationError>,
}

/// Auto-save configuration
#[derive(Debug, Clone)]
pub struct AutoSaveConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub max_idle_time_seconds: u64,
    pub save_on_change_count: Option<usize>,
    pub backup_directory: Option<PathBuf>,
    pub max_backup_files: usize,
    pub compress_backups: bool,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30,
            max_idle_time_seconds: 300, // 5 minutes
            save_on_change_count: Some(10),
            backup_directory: None,
            max_backup_files: 10,
            compress_backups: true,
        }
    }
}

/// Document state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentState {
    pub id: Uuid,
    pub file_path: Option<PathBuf>,
    pub is_modified: bool,
    pub last_saved: Option<u64>,
    pub last_modified: u64,
    pub change_count: usize,
    pub current_version: u64,
    pub content_size: usize,
    pub line_count: usize,
    pub word_count: usize,
    pub character_count: usize,
    pub encoding: String,
    pub line_endings: LineEnding,
    pub language: Option<String>,
}

/// Line ending types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LineEnding {
    Unix,    // LF (\n)
    Windows, // CRLF (\r\n)
    Mac,     // CR (\r)
    Mixed,   // Mixed line endings detected
}

/// Collaboration conflict detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetection {
    pub has_conflicts: bool,
    pub conflicts: Vec<Conflict>,
    pub remote_changes: Vec<DocumentChange>,
    pub local_changes: Vec<DocumentChange>,
    pub base_version: u64,
    pub resolution_strategy: ConflictResolutionStrategy,
}

/// Individual conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Uuid,
    pub position: TextRange,
    pub conflict_type: ConflictType,
    pub local_content: String,
    pub remote_content: String,
    pub base_content: Option<String>,
    pub severity: ConflictSeverity,
    pub auto_resolvable: bool,
}

/// Types of conflicts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictType {
    ContentOverlap,
    ConcurrentEdit,
    DeletionModification,
    FormatConflict,
    MetadataConflict,
}

/// Conflict severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Low,    // Can be auto-resolved
    Medium, // Requires user attention
    High,   // Critical conflict requiring manual resolution
}

/// Conflict resolution strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    Manual,
    PreferLocal,
    PreferRemote,
    Merge,
    ThreeWayMerge,
}

/// Large document chunk for memory-efficient handling
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub checksum: String,
    pub last_accessed: Instant,
    pub is_dirty: bool,
}

/// Progress callback for long-running operations
pub type ProgressCallback = Arc<dyn Fn(f32, String) + Send + Sync>;

/// Main document state manager
pub struct DocumentStateManager {
    document_id: Uuid,
    text_processor: Arc<TokioRwLock<MarkdownTextProcessor>>,
    markdown_processor: Arc<MarkdownProcessor>,
    
    // State tracking
    state: Arc<TokioRwLock<DocumentState>>,
    change_history: Arc<TokioRwLock<VecDeque<DocumentChange>>>,
    version_history: Arc<TokioRwLock<VecDeque<DocumentVersion>>>,
    
    // Auto-save
    auto_save_config: Arc<TokioRwLock<AutoSaveConfig>>,
    auto_save_handle: Arc<TokioMutex<Option<tokio::task::JoinHandle<()>>>>,
    last_activity: Arc<TokioRwLock<Instant>>,
    
    // Large document handling
    chunks: Arc<TokioRwLock<HashMap<Uuid, DocumentChunk>>>,
    chunk_size: usize,
    max_memory_usage: usize,
    
    // Collaboration
    conflict_detector: Arc<TokioRwLock<ConflictDetection>>,
    remote_change_buffer: Arc<TokioRwLock<Vec<DocumentChange>>>,
    
    // Event system
    change_sender: Arc<TokioMutex<Option<mpsc::UnboundedSender<DocumentChange>>>>,
    
    // Configuration
    max_history_size: usize,
    max_version_count: usize,
    enable_compression: bool,
}

impl DocumentStateManager {
    /// Create a new document state manager
    pub async fn new(document_id: Option<Uuid>) -> Result<Self, DocumentStateError> {
        let id = document_id.unwrap_or_else(Uuid::new_v4);
        let now = Self::current_timestamp();
        
        let initial_state = DocumentState {
            id,
            file_path: None,
            is_modified: false,
            last_saved: None,
            last_modified: now,
            change_count: 0,
            current_version: 1,
            content_size: 0,
            line_count: 1,
            word_count: 0,
            character_count: 0,
            encoding: "UTF-8".to_string(),
            line_endings: LineEnding::Unix,
            language: None,
        };

        let conflict_detection = ConflictDetection {
            has_conflicts: false,
            conflicts: Vec::new(),
            remote_changes: Vec::new(),
            local_changes: Vec::new(),
            base_version: 1,
            resolution_strategy: ConflictResolutionStrategy::Manual,
        };

        Ok(Self {
            document_id: id,
            text_processor: Arc::new(TokioRwLock::new(MarkdownTextProcessor::new())),
            markdown_processor: Arc::new(MarkdownProcessor::new()),
            state: Arc::new(TokioRwLock::new(initial_state)),
            change_history: Arc::new(TokioRwLock::new(VecDeque::new())),
            version_history: Arc::new(TokioRwLock::new(VecDeque::new())),
            auto_save_config: Arc::new(TokioRwLock::new(AutoSaveConfig::default())),
            auto_save_handle: Arc::new(TokioMutex::new(None)),
            last_activity: Arc::new(TokioRwLock::new(Instant::now())),
            chunks: Arc::new(TokioRwLock::new(HashMap::new())),
            chunk_size: 1000, // lines per chunk
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            conflict_detector: Arc::new(TokioRwLock::new(conflict_detection)),
            remote_change_buffer: Arc::new(TokioRwLock::new(Vec::new())),
            change_sender: Arc::new(TokioMutex::new(None)),
            max_history_size: 1000,
            max_version_count: 50,
            enable_compression: true,
        })
    }

    /// Load document from file
    pub async fn load_from_file(&self, file_path: &Path) -> Result<(), DocumentStateError> {
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))?;
        
        // Detect line endings
        let line_endings = self.detect_line_endings(&content);
        
        // Set content in text processor
        {
            let mut processor = self.text_processor.write().await;
            processor.set_content(content.clone());
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.file_path = Some(file_path.to_path_buf());
            state.content_size = content.len();
            state.line_count = content.lines().count();
            state.word_count = content.split_whitespace().count();
            state.character_count = content.chars().count();
            state.line_endings = line_endings;
            state.is_modified = false;
            state.last_saved = Some(Self::current_timestamp());
            state.last_modified = Self::current_timestamp();
        }
        
        // Create initial version
        self.create_version_snapshot("Initial load".to_string(), None).await?;
        
        // Chunk large documents
        if content.lines().count() > self.chunk_size {
            self.chunk_document(&content).await?;
        }

        Ok(())
    }

    /// Save document to file
    pub async fn save_to_file(&self, file_path: Option<&Path>) -> Result<(), DocumentStateError> {
        let content = {
            let processor = self.text_processor.read().await;
            processor.get_content().to_string()
        };
        
        let owned_path;
        let target_path = if let Some(path) = file_path {
            path
        } else {
            let state = self.state.read().await;
            match &state.file_path {
                Some(path) => {
                    owned_path = path.clone();
                    &owned_path
                },
                None => return Err(DocumentStateError::NoFilePath),
            }
        };
        
        // Apply line endings
        let content_with_endings = self.apply_line_endings(&content).await;
        
        // Create backup if configured
        if let Some(backup_dir) = &self.auto_save_config.read().await.backup_directory {
            self.create_backup(target_path, backup_dir).await?;
        }
        
        // Write file
        tokio::fs::write(target_path, content_with_endings).await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))?;
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.file_path = Some(target_path.to_path_buf());
            state.is_modified = false;
            state.last_saved = Some(Self::current_timestamp());
        }
        
        self.emit_change_event(DocumentChange {
            id: Uuid::new_v4(),
            timestamp: Self::current_timestamp(),
            change_type: ChangeType::StateSnapshot,
            position: TextRange::new(0, 0),
            old_content: None,
            new_content: None,
            author: None,
            checksum: self.calculate_checksum(&content),
            metadata: [("action".to_string(), "save".to_string())].into_iter().collect(),
        }).await;

        Ok(())
    }

    /// Get current document content
    pub async fn get_content(&self) -> String {
        let processor = self.text_processor.read().await;
        processor.get_content().to_string()
    }

    /// Set document content
    pub async fn set_content(&self, content: String) -> Result<(), DocumentStateError> {
        let old_content = {
            let processor = self.text_processor.read().await;
            processor.get_content().to_string()
        };
        
        {
            let mut processor = self.text_processor.write().await;
            processor.set_content(content.clone());
        }
        
        // Update state
        self.update_state_metrics(&content).await;
        
        // Record change
        let change = DocumentChange {
            id: Uuid::new_v4(),
            timestamp: Self::current_timestamp(),
            change_type: ChangeType::Replace,
            position: TextRange::new(0, old_content.len()),
            old_content: Some(old_content),
            new_content: Some(content.clone()),
            author: None,
            checksum: self.calculate_checksum(&content),
            metadata: HashMap::new(),
        };
        
        self.record_change(change.clone()).await?;
        self.emit_change_event(change).await;
        self.update_activity().await;
        
        Ok(())
    }

    /// Insert text at position
    pub async fn insert_text(&self, position: usize, text: &str) -> Result<(), DocumentStateError> {
        let _old_content = self.get_content().await;
        
        {
            let mut processor = self.text_processor.write().await;
            processor.insert_text(position, text)
                .map_err(DocumentStateError::TextProcessorError)?;
        }
        
        let new_content = self.get_content().await;
        self.update_state_metrics(&new_content).await;
        
        let change = DocumentChange {
            id: Uuid::new_v4(),
            timestamp: Self::current_timestamp(),
            change_type: ChangeType::Insert,
            position: TextRange::new(position, position + text.len()),
            old_content: None,
            new_content: Some(text.to_string()),
            author: None,
            checksum: self.calculate_checksum(&new_content),
            metadata: HashMap::new(),
        };
        
        self.record_change(change.clone()).await?;
        self.emit_change_event(change).await;
        self.update_activity().await;
        
        Ok(())
    }

    /// Delete text range
    pub async fn delete_range(&self, start: usize, end: usize) -> Result<String, DocumentStateError> {
        let _old_content = self.get_content().await;
        
        let deleted_text = {
            let mut processor = self.text_processor.write().await;
            processor.delete_range(start, end)
                .map_err(DocumentStateError::TextProcessorError)?
        };
        
        let new_content = self.get_content().await;
        self.update_state_metrics(&new_content).await;
        
        let change = DocumentChange {
            id: Uuid::new_v4(),
            timestamp: Self::current_timestamp(),
            change_type: ChangeType::Delete,
            position: TextRange::new(start, end),
            old_content: Some(deleted_text.clone()),
            new_content: None,
            author: None,
            checksum: self.calculate_checksum(&new_content),
            metadata: HashMap::new(),
        };
        
        self.record_change(change.clone()).await?;
        self.emit_change_event(change).await;
        self.update_activity().await;
        
        Ok(deleted_text)
    }

    /// Get document state information
    pub async fn get_state(&self) -> DocumentState {
        self.state.read().await.clone()
    }

    /// Get change history
    pub async fn get_change_history(&self, limit: Option<usize>) -> Vec<DocumentChange> {
        let history = self.change_history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get version history
    pub async fn get_version_history(&self) -> Vec<DocumentVersion> {
        let versions = self.version_history.read().await;
        versions.iter().cloned().collect()
    }

    /// Create a version snapshot
    pub async fn create_version_snapshot(&self, description: String, author: Option<String>) -> Result<DocumentVersion, DocumentStateError> {
        let content = self.get_content().await;
        let current_state = self.state.read().await;
        let changes = self.get_change_history(None).await;
        
        // Generate statistics and validation
        let statistics = self.markdown_processor.generate_statistics(&content).await
            .map_err(|e| DocumentStateError::ProcessingError(e.to_string()))?;
        
        let validation_errors = self.markdown_processor.validate(&content).await
            .map_err(|e| DocumentStateError::ProcessingError(e.to_string()))?;
        
        let version = DocumentVersion {
            id: Uuid::new_v4(),
            version_number: current_state.current_version,
            timestamp: Self::current_timestamp(),
            content: content.clone(),
            changes,
            checksum: self.calculate_checksum(&content),
            author,
            description,
            statistics: Some(statistics),
            validation_errors,
        };
        
        // Add to version history
        {
            let mut versions = self.version_history.write().await;
            versions.push_back(version.clone());
            
            // Limit version count
            while versions.len() > self.max_version_count {
                versions.pop_front();
            }
        }
        
        // Update state version number
        {
            let mut state = self.state.write().await;
            state.current_version += 1;
        }
        
        Ok(version)
    }

    /// Restore from version
    pub async fn restore_from_version(&self, version_id: Uuid) -> Result<(), DocumentStateError> {
        let version = {
            let versions = self.version_history.read().await;
            versions.iter()
                .find(|v| v.id == version_id)
                .cloned()
                .ok_or(DocumentStateError::VersionNotFound(version_id))?
        };
        
        self.set_content(version.content).await?;
        
        // Create new version for the restoration
        self.create_version_snapshot(
            format!("Restored from version {}", version.version_number),
            None
        ).await?;
        
        Ok(())
    }

    /// Configure auto-save
    pub async fn configure_auto_save(&self, config: AutoSaveConfig) -> Result<(), DocumentStateError> {
        let was_enabled = self.auto_save_config.read().await.enabled;
        *self.auto_save_config.write().await = config.clone();
        
        if config.enabled && !was_enabled {
            self.start_auto_save().await?;
        } else if !config.enabled && was_enabled {
            self.stop_auto_save().await;
        }
        
        Ok(())
    }

    /// Start auto-save background task
    async fn start_auto_save(&self) -> Result<(), DocumentStateError> {
        let mut handle_guard = self.auto_save_handle.lock().await;
        
        if handle_guard.is_some() {
            return Ok(()); // Already running
        }
        
        let state_manager = DocumentStateManagerWeak {
            document_id: self.document_id,
            text_processor: Arc::downgrade(&self.text_processor),
            state: Arc::downgrade(&self.state),
            auto_save_config: Arc::downgrade(&self.auto_save_config),
            last_activity: Arc::downgrade(&self.last_activity),
        };
        
        let handle = tokio::spawn(async move {
            state_manager.auto_save_loop().await;
        });
        
        *handle_guard = Some(handle);
        Ok(())
    }

    /// Stop auto-save background task
    async fn stop_auto_save(&self) {
        let mut handle_guard = self.auto_save_handle.lock().await;
        
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }

    /// Handle remote changes for collaboration
    pub async fn apply_remote_changes(&self, remote_changes: Vec<DocumentChange>) -> Result<ConflictDetection, DocumentStateError> {
        // Buffer remote changes
        {
            let mut buffer = self.remote_change_buffer.write().await;
            buffer.extend(remote_changes.clone());
        }
        
        // Get local changes since last sync
        let local_changes = self.get_change_history(None).await;
        
        // Detect conflicts
        let conflicts = self.detect_conflicts(&local_changes, &remote_changes).await;
        
        let conflict_detection = ConflictDetection {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
            remote_changes,
            local_changes,
            base_version: self.state.read().await.current_version,
            resolution_strategy: ConflictResolutionStrategy::Manual,
        };
        
        *self.conflict_detector.write().await = conflict_detection.clone();
        
        Ok(conflict_detection)
    }

    /// Resolve conflicts with specified strategy
    pub async fn resolve_conflicts(&self, strategy: ConflictResolutionStrategy) -> Result<(), DocumentStateError> {
        let mut conflict_detector = self.conflict_detector.write().await;
        
        if !conflict_detector.has_conflicts {
            return Ok(());
        }
        
        match strategy {
            ConflictResolutionStrategy::PreferLocal => {
                // Keep local changes, ignore remote
                conflict_detector.has_conflicts = false;
                conflict_detector.conflicts.clear();
            },
            ConflictResolutionStrategy::PreferRemote => {
                // Apply remote changes, discard conflicting local changes
                for remote_change in &conflict_detector.remote_changes {
                    self.apply_remote_change(remote_change).await?;
                }
                conflict_detector.has_conflicts = false;
                conflict_detector.conflicts.clear();
            },
            ConflictResolutionStrategy::Merge => {
                // Attempt automatic three-way merge
                self.perform_three_way_merge(&mut conflict_detector).await?;
            },
            ConflictResolutionStrategy::Manual => {
                // Requires manual resolution
                return Err(DocumentStateError::ManualResolutionRequired);
            },
            ConflictResolutionStrategy::ThreeWayMerge => {
                self.perform_three_way_merge(&mut conflict_detector).await?;
            },
        }
        
        Ok(())
    }

    /// Subscribe to document change events
    pub async fn subscribe_to_changes(&self) -> mpsc::UnboundedReceiver<DocumentChange> {
        let (sender, receiver) = mpsc::unbounded_channel();
        *self.change_sender.lock().await = Some(sender);
        receiver
    }

    /// Get document processing statistics
    pub async fn get_statistics(&self) -> Result<ProcessingStatistics, DocumentStateError> {
        let content = self.get_content().await;
        self.markdown_processor.generate_statistics(&content).await
            .map_err(|e| DocumentStateError::ProcessingError(e.to_string()))
    }

    /// Validate document content
    pub async fn validate(&self) -> Result<Vec<ValidationError>, DocumentStateError> {
        let content = self.get_content().await;
        self.markdown_processor.validate(&content).await
            .map_err(|e| DocumentStateError::ProcessingError(e.to_string()))
    }

    /// Get memory usage information
    pub async fn get_memory_usage(&self) -> MemoryUsageInfo {
        let content_size = self.get_content().await.len();
        let chunks = self.chunks.read().await;
        let chunk_memory: usize = chunks.values()
            .map(|chunk| chunk.content.len())
            .sum();
        
        let history_memory = {
            let history = self.change_history.read().await;
            history.iter()
                .map(|change| {
                    change.old_content.as_ref().map(|s| s.len()).unwrap_or(0) +
                    change.new_content.as_ref().map(|s| s.len()).unwrap_or(0)
                })
                .sum::<usize>()
        };
        
        let version_memory = {
            let versions = self.version_history.read().await;
            versions.iter()
                .map(|version| version.content.len())
                .sum::<usize>()
        };
        
        MemoryUsageInfo {
            content_size,
            chunk_memory,
            history_memory,
            version_memory,
            total_memory: content_size + chunk_memory + history_memory + version_memory,
            chunk_count: chunks.len(),
            max_memory_limit: self.max_memory_usage,
        }
    }

    /// Optimize memory usage
    pub async fn optimize_memory(&self) -> Result<(), DocumentStateError> {
        // Unload unused chunks
        {
            let mut chunks = self.chunks.write().await;
            let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
            chunks.retain(|_, chunk| chunk.last_accessed > cutoff);
        }
        
        // Compress old versions if enabled
        if self.enable_compression {
            self.compress_old_versions().await?;
        }
        
        // Trim history if over limit
        {
            let mut history = self.change_history.write().await;
            while history.len() > self.max_history_size {
                history.pop_front();
            }
        }
        
        Ok(())
    }

    // Private helper methods
    
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn calculate_checksum(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn detect_line_endings(&self, content: &str) -> LineEnding {
        let crlf_count = content.matches("\r\n").count();
        let lf_count = content.matches('\n').count() - crlf_count;
        let cr_count = content.matches('\r').count() - crlf_count;
        
        match (crlf_count > 0, lf_count > 0, cr_count > 0) {
            (true, false, false) => LineEnding::Windows,
            (false, true, false) => LineEnding::Unix,
            (false, false, true) => LineEnding::Mac,
            _ => LineEnding::Mixed,
        }
    }

    async fn apply_line_endings(&self, content: &str) -> String {
        let line_ending = &self.state.read().await.line_endings;
        match line_ending {
            LineEnding::Windows => content.replace('\n', "\r\n"),
            LineEnding::Mac => content.replace('\n', "\r"),
            LineEnding::Unix | LineEnding::Mixed => content.to_string(),
        }
    }

    async fn update_state_metrics(&self, content: &str) {
        let mut state = self.state.write().await;
        state.content_size = content.len();
        state.line_count = content.lines().count();
        state.word_count = content.split_whitespace().count();
        state.character_count = content.chars().count();
        state.is_modified = true;
        state.last_modified = Self::current_timestamp();
        state.change_count += 1;
    }

    async fn record_change(&self, change: DocumentChange) -> Result<(), DocumentStateError> {
        let mut history = self.change_history.write().await;
        history.push_back(change);
        
        // Limit history size
        while history.len() > self.max_history_size {
            history.pop_front();
        }
        
        // Check if auto-save should trigger
        let config = self.auto_save_config.read().await;
        if let Some(count_threshold) = config.save_on_change_count {
            if history.len() % count_threshold == 0 {
                // Trigger save (this would be handled by the auto-save system)
                self.update_activity().await;
            }
        }
        
        Ok(())
    }

    async fn emit_change_event(&self, change: DocumentChange) {
        if let Some(sender) = &*self.change_sender.lock().await {
            let _ = sender.send(change);
        }
    }

    async fn update_activity(&self) {
        *self.last_activity.write().await = Instant::now();
    }

    async fn chunk_document(&self, content: &str) -> Result<(), DocumentStateError> {
        let lines: Vec<&str> = content.lines().collect();
        let mut chunks = self.chunks.write().await;
        chunks.clear();
        
        for (chunk_index, chunk_lines) in lines.chunks(self.chunk_size).enumerate() {
            let start_line = chunk_index * self.chunk_size;
            let end_line = start_line + chunk_lines.len();
            let chunk_content = chunk_lines.join("\n");
            
            let chunk = DocumentChunk {
                id: Uuid::new_v4(),
                start_line,
                end_line,
                content: chunk_content.clone(),
                checksum: self.calculate_checksum(&chunk_content),
                last_accessed: Instant::now(),
                is_dirty: false,
            };
            
            chunks.insert(chunk.id, chunk);
        }
        
        Ok(())
    }

    async fn create_backup(&self, source_path: &Path, backup_dir: &Path) -> Result<(), DocumentStateError> {
        tokio::fs::create_dir_all(backup_dir).await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))?;
        
        let timestamp = Self::current_timestamp();
        let filename = source_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document");
        let backup_filename = format!("{}_{}.bak", filename, timestamp);
        let backup_path = backup_dir.join(backup_filename);
        
        tokio::fs::copy(source_path, backup_path).await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))?;
        
        // Clean up old backups
        self.cleanup_old_backups(backup_dir).await?;
        
        Ok(())
    }

    async fn cleanup_old_backups(&self, backup_dir: &Path) -> Result<(), DocumentStateError> {
        let max_backups = self.auto_save_config.read().await.max_backup_files;
        
        let mut entries = tokio::fs::read_dir(backup_dir).await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))?;
        
        let mut backup_files = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| DocumentStateError::IoError(e.to_string()))? {
            
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".bak") {
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            backup_files.push((entry.path(), modified));
                        }
                    }
                }
            }
        }
        
        // Sort by modification time (newest first)
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Remove excess backups
        for (path, _) in backup_files.into_iter().skip(max_backups) {
            let _ = tokio::fs::remove_file(path).await;
        }
        
        Ok(())
    }

    async fn detect_conflicts(&self, local_changes: &[DocumentChange], remote_changes: &[DocumentChange]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        
        for local_change in local_changes {
            for remote_change in remote_changes {
                if self.changes_overlap(local_change, remote_change) {
                    let conflict = Conflict {
                        id: Uuid::new_v4(),
                        position: TextRange::new(
                            local_change.position.start.min(remote_change.position.start),
                            local_change.position.end.max(remote_change.position.end),
                        ),
                        conflict_type: self.determine_conflict_type(local_change, remote_change),
                        local_content: local_change.new_content.clone().unwrap_or_default(),
                        remote_content: remote_change.new_content.clone().unwrap_or_default(),
                        base_content: local_change.old_content.clone(),
                        severity: ConflictSeverity::Medium,
                        auto_resolvable: false,
                    };
                    conflicts.push(conflict);
                }
            }
        }
        
        conflicts
    }

    fn changes_overlap(&self, change1: &DocumentChange, change2: &DocumentChange) -> bool {
        change1.position.overlaps(&change2.position)
    }

    fn determine_conflict_type(&self, local_change: &DocumentChange, remote_change: &DocumentChange) -> ConflictType {
        match (&local_change.change_type, &remote_change.change_type) {
            (ChangeType::Delete, ChangeType::Replace) => ConflictType::DeletionModification,
            (ChangeType::Replace, ChangeType::Delete) => ConflictType::DeletionModification,
            (ChangeType::Format, _) | (_, ChangeType::Format) => ConflictType::FormatConflict,
            _ => ConflictType::ContentOverlap,
        }
    }

    async fn apply_remote_change(&self, remote_change: &DocumentChange) -> Result<(), DocumentStateError> {
        match remote_change.change_type {
            ChangeType::Insert => {
                if let Some(content) = &remote_change.new_content {
                    self.insert_text(remote_change.position.start, content).await?;
                }
            },
            ChangeType::Delete => {
                self.delete_range(remote_change.position.start, remote_change.position.end).await?;
            },
            ChangeType::Replace => {
                if let Some(content) = &remote_change.new_content {
                    self.delete_range(remote_change.position.start, remote_change.position.end).await?;
                    self.insert_text(remote_change.position.start, content).await?;
                }
            },
            _ => {} // Handle other change types as needed
        }
        Ok(())
    }

    async fn perform_three_way_merge(&self, conflict_detector: &mut ConflictDetection) -> Result<(), DocumentStateError> {
        // This is a simplified three-way merge implementation
        // In practice, you'd want a more sophisticated algorithm
        
        for conflict in &conflict_detector.conflicts {
            if conflict.auto_resolvable {
                // Apply automatic resolution based on conflict type
                match conflict.conflict_type {
                    ConflictType::FormatConflict => {
                        // Prefer local formatting
                        continue;
                    },
                    _ => {
                        // For other types, attempt simple merge
                        let merged_content = self.simple_merge(
                            &conflict.local_content,
                            &conflict.remote_content,
                            conflict.base_content.as_deref(),
                        );
                        
                        self.delete_range(conflict.position.start, conflict.position.end).await?;
                        self.insert_text(conflict.position.start, &merged_content).await?;
                    }
                }
            }
        }
        
        conflict_detector.has_conflicts = false;
        conflict_detector.conflicts.clear();
        
        Ok(())
    }

    fn simple_merge(&self, local: &str, remote: &str, base: Option<&str>) -> String {
        // Very basic merge - in practice you'd want a proper diff3 algorithm
        if local == remote {
            local.to_string()
        } else if let Some(base_content) = base {
            if local == base_content {
                remote.to_string()
            } else if remote == base_content {
                local.to_string()
            } else {
                format!("<<<<<<< LOCAL\n{}\n=======\n{}\n>>>>>>> REMOTE", local, remote)
            }
        } else {
            format!("<<<<<<< LOCAL\n{}\n=======\n{}\n>>>>>>> REMOTE", local, remote)
        }
    }

    async fn compress_old_versions(&self) -> Result<(), DocumentStateError> {
        // Placeholder for version compression logic
        // In practice, you'd compress old versions using a compression algorithm
        Ok(())
    }
}

/// Weak references for auto-save background task
struct DocumentStateManagerWeak {
    document_id: Uuid,
    text_processor: Weak<TokioRwLock<MarkdownTextProcessor>>,
    state: Weak<TokioRwLock<DocumentState>>,
    auto_save_config: Weak<TokioRwLock<AutoSaveConfig>>,
    last_activity: Weak<TokioRwLock<Instant>>,
}

impl DocumentStateManagerWeak {
    async fn auto_save_loop(&self) {
        loop {
            // Check if references are still valid
            let (config, state, last_activity) = match (
                self.auto_save_config.upgrade(),
                self.state.upgrade(),
                self.last_activity.upgrade(),
            ) {
                (Some(config), Some(state), Some(activity)) => (config, state, activity),
                _ => break, // Manager was dropped
            };
            
            let config_guard = config.read().await;
            if !config_guard.enabled {
                break;
            }
            
            let interval_duration = Duration::from_secs(config_guard.interval_seconds);
            let max_idle = Duration::from_secs(config_guard.max_idle_time_seconds);
            drop(config_guard);
            
            // Wait for the interval
            sleep(interval_duration).await;
            
            // Check if we should save
            let should_save = {
                let state_guard = state.read().await;
                let activity_guard = last_activity.read().await;
                
                state_guard.is_modified && 
                activity_guard.elapsed() < max_idle
            };
            
            if should_save {
                // In a real implementation, you'd trigger a save here
                // For now, we just update the activity timestamp
                println!("Auto-save triggered for document {}", self.document_id);
            }
        }
    }
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryUsageInfo {
    pub content_size: usize,
    pub chunk_memory: usize,
    pub history_memory: usize,
    pub version_memory: usize,
    pub total_memory: usize,
    pub chunk_count: usize,
    pub max_memory_limit: usize,
}

/// Document state manager errors
#[derive(Debug, thiserror::Error)]
pub enum DocumentStateError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Text processor error: {0}")]
    TextProcessorError(#[from] TextProcessorError),
    
    #[error("Processing error: {0}")]
    ProcessingError(String),
    
    #[error("No file path specified")]
    NoFilePath,
    
    #[error("Version not found: {0}")]
    VersionNotFound(Uuid),
    
    #[error("Manual conflict resolution required")]
    ManualResolutionRequired,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_document_state_manager_creation() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        let state = manager.get_state().await;
        assert_eq!(state.change_count, 0);
        assert!(!state.is_modified);
    }

    #[tokio::test]
    async fn test_content_operations() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        manager.set_content("Hello, World!".to_string()).await.unwrap();
        let content = manager.get_content().await;
        assert_eq!(content, "Hello, World!");
        
        let state = manager.get_state().await;
        assert!(state.is_modified);
        assert_eq!(state.character_count, 13);
    }

    #[tokio::test]
    async fn test_text_operations() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        manager.insert_text(0, "Hello").await.unwrap();
        manager.insert_text(5, " World").await.unwrap();
        
        let content = manager.get_content().await;
        assert_eq!(content, "Hello World");
        
        let deleted = manager.delete_range(5, 11).await.unwrap();
        assert_eq!(deleted, " World");
        
        let final_content = manager.get_content().await;
        assert_eq!(final_content, "Hello");
    }

    #[tokio::test]
    async fn test_version_management() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        manager.set_content("Version 1".to_string()).await.unwrap();
        let version1 = manager.create_version_snapshot("First version".to_string(), None).await.unwrap();
        
        manager.set_content("Version 2".to_string()).await.unwrap();
        let version2 = manager.create_version_snapshot("Second version".to_string(), None).await.unwrap();
        
        let versions = manager.get_version_history().await;
        assert_eq!(versions.len(), 2);
        
        // Restore to version 1
        manager.restore_from_version(version1.id).await.unwrap();
        let content = manager.get_content().await;
        assert_eq!(content, "Version 1");
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        
        // Create test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Test Document\n\nThis is a test.").unwrap();
        
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        // Load from file
        manager.load_from_file(&file_path).await.unwrap();
        let content = manager.get_content().await;
        assert!(content.contains("# Test Document"));
        
        // Modify and save
        manager.set_content("# Modified Document\n\nThis is modified.".to_string()).await.unwrap();
        manager.save_to_file(Some(&file_path)).await.unwrap();
        
        // Verify file was saved
        let saved_content = std::fs::read_to_string(&file_path).unwrap();
        assert!(saved_content.contains("# Modified Document"));
    }

    #[tokio::test]
    async fn test_change_history() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        manager.insert_text(0, "Hello").await.unwrap();
        manager.insert_text(5, " World").await.unwrap();
        manager.delete_range(5, 11).await.unwrap();
        
        let history = manager.get_change_history(None).await;
        assert_eq!(history.len(), 3);
        
        // Check change types
        assert_eq!(history[0].change_type, ChangeType::Delete);
        assert_eq!(history[1].change_type, ChangeType::Insert);
        assert_eq!(history[2].change_type, ChangeType::Insert);
    }

    #[tokio::test]
    async fn test_statistics_and_validation() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        let markdown_content = "# Heading\n\nParagraph with [link](url) and ![image](img.jpg).\n\n## Another heading";
        manager.set_content(markdown_content.to_string()).await.unwrap();
        
        let stats = manager.get_statistics().await.unwrap();
        assert_eq!(stats.heading_count.get(&1), Some(&1));
        assert_eq!(stats.heading_count.get(&2), Some(&1));
        assert_eq!(stats.link_count, 1);
        assert_eq!(stats.image_count, 1);
        
        let validation_errors = manager.validate().await.unwrap();
        // Should have some validation warnings for the empty URL and image
        assert!(!validation_errors.is_empty());
    }

    #[tokio::test]
    async fn test_memory_usage() {
        let manager = DocumentStateManager::new(None).await.unwrap();
        
        let large_content = "x".repeat(10000);
        manager.set_content(large_content).await.unwrap();
        
        let memory_info = manager.get_memory_usage().await;
        assert_eq!(memory_info.content_size, 10000);
        assert!(memory_info.total_memory >= 10000);
    }
}