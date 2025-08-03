//! Commit Message Builder
//! 
//! Standardizes commit messages for translation workflows with proper metadata.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

/// Builder for standardized commit messages
#[derive(Debug, Default)]
pub struct CommitMessageBuilder {
    commit_type: Option<CommitType>,
    scope: Option<String>,
    description: String,
    body: Vec<String>,
    metadata: HashMap<String, String>,
    breaking_change: bool,
}

/// Types of commits in the translation workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitType {
    /// Translation work commit
    Translate,
    /// Review feedback commit
    Review,
    /// Task management commit
    Task,
    /// Initial repository setup
    Init,
    /// Feature work (non-translation)
    Feat,
    /// Bug fixes
    Fix,
    /// Documentation updates
    Docs,
    /// Refactoring
    Refactor,
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CommitType::Translate => "translate",
            CommitType::Review => "review",
            CommitType::Task => "task",
            CommitType::Init => "init",
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Docs => "docs",
            CommitType::Refactor => "refactor",
        };
        write!(f, "{}", s)
    }
}

impl CommitMessageBuilder {
    /// Create a new commit message builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the commit type
    pub fn commit_type(mut self, commit_type: CommitType) -> Self {
        self.commit_type = Some(commit_type);
        self
    }

    /// Set the scope (language, chapter, etc.)
    pub fn scope<S: Into<String>>(mut self, scope: S) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Set the main description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = description.into();
        self
    }

    /// Add a body line
    pub fn body_line<S: Into<String>>(mut self, line: S) -> Self {
        self.body.push(line.into());
        self
    }

    /// Add multiple body lines
    pub fn body_lines<I, S>(mut self, lines: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for line in lines {
            self.body.push(line.into());
        }
        self
    }

    /// Add metadata (key-value pairs at the end of commit message)
    pub fn metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Mark as breaking change
    pub fn breaking_change(mut self) -> Self {
        self.breaking_change = true;
        self
    }

    /// Build the final commit message
    pub fn build(self) -> String {
        let mut message = String::new();

        // Format: type(scope): description
        if let Some(commit_type) = self.commit_type {
            message.push_str(&commit_type.to_string());
            
            if let Some(scope) = self.scope {
                message.push_str(&format!("({})", scope));
            }
            
            if self.breaking_change {
                message.push('!');
            }
            
            message.push_str(": ");
        }

        message.push_str(&self.description);

        // Add body if present
        if !self.body.is_empty() {
            message.push_str("\n\n");
            message.push_str(&self.body.join("\n"));
        }

        // Add metadata
        if !self.metadata.is_empty() {
            message.push_str("\n\n");
            for (key, value) in &self.metadata {
                message.push_str(&format!("{}: {}\n", key, value));
            }
        }

        message
    }
}

/// Predefined commit message templates for common operations
pub struct CommitTemplates;

impl CommitTemplates {
    /// Template for starting a translation session
    pub fn start_translation_session(
        chapter: &str,
        language: &str,
        translator: &str,
        session_id: Uuid,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope(format!("{}/{}", chapter, language))
            .description("start translation session")
            .body_lines([
                format!("Initialize {} translation work for {}", language, chapter),
                "".to_string(),
                "This branch will contain all translation work for this chapter/language combination.".to_string(),
            ])
            .metadata("Translator", translator)
            .metadata("Session-ID", session_id.to_string())
            .metadata("Chapter", chapter)
            .metadata("Language", language)
            .metadata("Status", "In Progress")
            .build()
    }

    /// Template for auto-save commits during translation
    pub fn auto_save_translation(
        language: &str,
        session_id: Uuid,
        word_count: Option<usize>,
    ) -> String {
        let mut builder = CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope(language)
            .description("auto-save translation progress")
            .metadata("Session-ID", session_id.to_string())
            .metadata("Auto-Save", "true")
            .metadata("Timestamp", Utc::now().to_rfc3339());

        if let Some(count) = word_count {
            builder = builder.metadata("Word-Count", count.to_string());
        }

        builder.build()
    }

    /// Template for completing translation work
    pub fn complete_translation(
        chapter: &str,
        language: &str,
        translator: &str,
        session_id: Uuid,
        summary: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope(format!("{}/{}", chapter, language))
            .description("complete translation work")
            .body_lines([
                format!("Ready for review: {}", summary),
                "".to_string(),
                "All translation units have been completed and are ready for reviewer feedback.".to_string(),
            ])
            .metadata("Translator", translator)
            .metadata("Session-ID", session_id.to_string())
            .metadata("Status", "Completed")
            .metadata("Review-Required", "true")
            .build()
    }

    /// Template for reviewer feedback
    pub fn review_feedback(
        chapter: &str,
        language: &str,
        reviewer: &str,
        translator: &str,
        status: &str,
        feedback_summary: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Review)
            .scope(format!("{}/{}", chapter, language))
            .description(format!("{} translation review", status.to_lowercase()))
            .body_line(feedback_summary)
            .metadata("Reviewer", reviewer)
            .metadata("Translator", translator)
            .metadata("Status", status)
            .metadata("Review-Date", Utc::now().to_rfc3339())
            .build()
    }

    /// Template for creating todos
    pub fn create_todo(
        title: &str,
        created_by: &str,
        assigned_to: Option<&str>,
        priority: &str,
        context: &str,
    ) -> String {
        let mut builder = CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .description(format!("add todo: {}", title))
            .metadata("Created-By", created_by)
            .metadata("Priority", priority)
            .metadata("Context", context);

        if let Some(assignee) = assigned_to {
            builder = builder.metadata("Assigned-To", assignee);
        }

        builder.build()
    }

    /// Template for completing todos
    pub fn complete_todo(
        title: &str,
        completed_by: &str,
        resolution: Option<&str>,
    ) -> String {
        let mut builder = CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .description(format!("complete todo: {}", title))
            .metadata("Completed-By", completed_by)
            .metadata("Status", "Completed");

        if let Some(res) = resolution {
            builder = builder.body_line(format!("Resolution: {}", res));
        }

        builder.build()
    }

    /// Template for approval and merge
    pub fn approve_and_merge(
        chapter: &str,
        language: &str,
        reviewer: &str,
        translator: &str,
        pr_id: u64,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Review)
            .scope(format!("{}/{}", chapter, language))
            .description("approve and merge translation")
            .body_lines([
                format!("Approved translation work by {}", translator),
                format!("Merged from PR #{}", pr_id),
            ])
            .metadata("Reviewer", reviewer)
            .metadata("Translator", translator)
            .metadata("PR-ID", pr_id.to_string())
            .metadata("Status", "Approved")
            .metadata("Merged-At", Utc::now().to_rfc3339())
            .build()
    }

    /// Template for terminology research
    pub fn terminology_research(
        term: &str,
        language: &str,
        researcher: &str,
        context: &str,
        resolution: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .scope(language)
            .description(format!("research terminology: {}", term))
            .body_lines([
                format!("Context: {}", context),
                format!("Resolution: {}", resolution),
            ])
            .metadata("Researcher", researcher)
            .metadata("Term", term)
            .metadata("Language", language)
            .metadata("Type", "terminology")
            .build()
    }

    /// Template for screenshot updates
    pub fn update_screenshots(
        chapter: &str,
        screenshot_count: u32,
        updated_by: &str,
        language: Option<&str>,
    ) -> String {
        let scope = if let Some(lang) = language {
            format!("{}/{}", chapter, lang)
        } else {
            chapter.to_string()
        };

        CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .scope(scope)
            .description("update screenshots")
            .body_line(format!("Updated {} screenshots for chapter", screenshot_count))
            .metadata("Updated-By", updated_by)
            .metadata("Screenshot-Count", screenshot_count.to_string())
            .metadata("Type", "screenshot")
            .build()
    }

    /// Template for quality score updates
    pub fn update_quality_score(
        chapter: &str,
        language: &str,
        score: f32,
        reviewer: &str,
        notes: Option<&str>,
    ) -> String {
        let mut builder = CommitMessageBuilder::new()
            .commit_type(CommitType::Review)
            .scope(format!("{}/{}", chapter, language))
            .description("update quality score")
            .metadata("Reviewer", reviewer)
            .metadata("Quality-Score", score.to_string())
            .metadata("Language", language);

        if let Some(notes) = notes {
            builder = builder.body_line(format!("Notes: {}", notes));
        }

        builder.build()
    }

    /// Template for TOML data synchronization
    pub fn sync_toml_data(
        chapter: &str,
        language: &str,
        units_updated: u32,
        sync_type: &str,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope(format!("{}/{}", chapter, language))
            .description("sync TOML data from markdown")
            .body_lines([
                format!("Synchronized {} translation units", units_updated),
                format!("Sync type: {}", sync_type),
                "".to_string(),
                "TOML data updated to reflect current markdown content.".to_string(),
            ])
            .metadata("Updated-By", user)
            .metadata("Units-Updated", units_updated.to_string())
            .metadata("Sync-Type", sync_type)
            .metadata("Language", language)
            .build()
    }

    /// Template for session recovery
    pub fn recover_session(
        chapter: &str,
        language: &str,
        session_id: uuid::Uuid,
        recovery_reason: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .scope(format!("{}/{}", chapter, language))
            .description("recover translation session")
            .body_lines([
                format!("Recovered session after {}", recovery_reason),
                "".to_string(),
                "Session state restored from previous work.".to_string(),
            ])
            .metadata("Session-ID", session_id.to_string())
            .metadata("Recovery-Reason", recovery_reason)
            .metadata("Recovered-At", chrono::Utc::now().to_rfc3339())
            .build()
    }

    /// Template for bulk translation operations
    pub fn bulk_translation_update(
        chapter: &str,
        language: &str,
        operation_type: &str,
        units_affected: u32,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope(format!("{}/{}", chapter, language))
            .description(format!("bulk {}", operation_type))
            .body_lines([
                format!("Performed bulk {} on {} units", operation_type, units_affected),
                "".to_string(),
                "Multiple translation units updated in single operation.".to_string(),
            ])
            .metadata("Operation-Type", operation_type)
            .metadata("Units-Affected", units_affected.to_string())
            .metadata("Performed-By", user)
            .metadata("Language", language)
            .build()
    }

    /// Template for translation status changes
    pub fn translation_status_change(
        chapter: &str,
        language: &str,
        unit_id: &str,
        old_status: &str,
        new_status: &str,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .scope(format!("{}/{}", chapter, language))
            .description(format!("update status: {} -> {}", old_status, new_status))
            .body_line(format!("Translation unit {} status changed", unit_id))
            .metadata("Unit-ID", unit_id)
            .metadata("Old-Status", old_status)
            .metadata("New-Status", new_status)
            .metadata("Updated-By", user)
            .metadata("Language", language)
            .build()
    }

    /// Template for chapter structure changes
    pub fn chapter_structure_update(
        chapter: &str,
        change_type: &str,
        description: &str,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Refactor)
            .scope(chapter)
            .description(format!("chapter structure: {}", change_type))
            .body_lines([
                description.to_string(),
                "".to_string(),
                "Chapter structure updated to improve organization.".to_string(),
            ])
            .metadata("Change-Type", change_type)
            .metadata("Updated-By", user)
            .metadata("Chapter", chapter)
            .build()
    }

    /// Template for translation unit reordering
    pub fn reorder_translation_units(
        chapter: &str,
        units_reordered: u32,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Refactor)
            .scope(chapter)
            .description("reorder translation units")
            .body_lines([
                format!("Reordered {} translation units", units_reordered),
                "".to_string(),
                "Units reorganized for better logical flow.".to_string(),
            ])
            .metadata("Units-Reordered", units_reordered.to_string())
            .metadata("Updated-By", user)
            .metadata("Type", "reorder")
            .build()
    }

    /// Template for metadata cleanup
    pub fn cleanup_metadata(
        chapter: &str,
        cleanup_type: &str,
        items_cleaned: u32,
        user: &str,
    ) -> String {
        CommitMessageBuilder::new()
            .commit_type(CommitType::Task)
            .scope(chapter)
            .description(format!("cleanup {}", cleanup_type))
            .body_lines([
                format!("Cleaned up {} {} items", items_cleaned, cleanup_type),
                "".to_string(),
                "Metadata cleanup to improve data quality.".to_string(),
            ])
            .metadata("Cleanup-Type", cleanup_type)
            .metadata("Items-Cleaned", items_cleaned.to_string())
            .metadata("Cleaned-By", user)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_commit_message() {
        let message = CommitMessageBuilder::new()
            .commit_type(CommitType::Translate)
            .scope("intro/de")
            .description("update German translation")
            .build();

        assert_eq!(message, "translate(intro/de): update German translation");
    }

    #[test]
    fn test_commit_with_body_and_metadata() {
        let message = CommitMessageBuilder::new()
            .commit_type(CommitType::Review)
            .scope("de")
            .description("approve translation")
            .body_line("All requirements met")
            .metadata("Reviewer", "john.doe")
            .metadata("Status", "Approved")
            .build();

        let expected = "review(de): approve translation\n\nAll requirements met\n\nReviewer: john.doe\nStatus: Approved\n";
        assert_eq!(message, expected);
    }

    #[test]
    fn test_breaking_change() {
        let message = CommitMessageBuilder::new()
            .commit_type(CommitType::Feat)
            .scope("api")
            .description("change translation format")
            .breaking_change()
            .build();

        assert_eq!(message, "feat(api)!: change translation format");
    }

    #[test]
    fn test_template_start_session() {
        let session_id = Uuid::new_v4();
        let message = CommitTemplates::start_translation_session(
            "introduction",
            "de",
            "translator.user",
            session_id,
        );

        assert!(message.contains("translate(introduction/de): start translation session"));
        assert!(message.contains("Translator: translator.user"));
        assert!(message.contains(&session_id.to_string()));
    }
}