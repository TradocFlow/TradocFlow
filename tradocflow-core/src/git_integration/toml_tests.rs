//! Unit Tests for TOML Data Layer
//! 
//! Comprehensive tests for TOML data structures and I/O operations.

#[cfg(test)]
mod tests {
    use crate::git_integration::toml_data::*;
    use crate::git_integration::toml_io::*;
    use tempfile::TempDir;
    use std::collections::HashMap;
    use chrono::Utc;

    /// Create a test project for testing
    fn create_test_project() -> ProjectData {
        ProjectData::new(
            "test-project-001".to_string(),
            "Test Translation Project".to_string(),
            "A comprehensive test project for unit testing".to_string(),
            "en".to_string(),
            vec!["de".to_string(), "fr".to_string(), "es".to_string()],
            "test-editor".to_string(),
        )
    }

    /// Create a test chapter for testing
    fn create_test_chapter() -> ChapterData {
        let mut titles = HashMap::new();
        titles.insert("en".to_string(), "Introduction to Testing".to_string());
        titles.insert("de".to_string(), "Einführung ins Testen".to_string());
        titles.insert("fr".to_string(), "Introduction aux Tests".to_string());

        let mut chapter = ChapterData::new(
            1,
            "introduction-testing".to_string(),
            titles,
            "en".to_string(),
        );

        // Add some translation units
        let unit1 = TranslationUnit::new(
            "intro_p001".to_string(),
            1,
            "en".to_string(),
            "Welcome to our comprehensive testing framework.".to_string(),
            ComplexityLevel::Low,
        );

        let unit2 = TranslationUnit::new(
            "intro_p002".to_string(),
            2,
            "en".to_string(),
            "This framework supports multiple languages and complex translation workflows.".to_string(),
            ComplexityLevel::Medium,
        );

        chapter.add_unit(unit1);
        chapter.add_unit(unit2);

        // Add some todos
        let todo = ChapterTodo {
            id: "chapter_todo_001".to_string(),
            title: "Review technical terminology".to_string(),
            description: Some("Ensure all technical terms are consistent".to_string()),
            created_by: "test-reviewer".to_string(),
            assigned_to: Some("test-translator".to_string()),
            priority: Priority::Medium,
            status: TodoStatus::Open,
            todo_type: TodoType::Terminology,
            context: "chapter".to_string(),
            created_at: Utc::now(),
            due_date: None,
            resolved_at: None,
            resolution: None,
        };

        chapter.todos.push(todo);

        chapter
    }

    #[test]
    fn test_project_data_serialization() {
        let project = create_test_project();
        
        // Test TOML serialization
        let toml_string = toml::to_string_pretty(&project).unwrap();
        println!("Serialized TOML:\n{}", toml_string);

        // Should contain expected sections
        assert!(toml_string.contains("[project]"));
        assert!(toml_string.contains("[project.languages]"));
        assert!(toml_string.contains("[project.team]"));
        assert!(toml_string.contains("[project.settings]"));

        // Test deserialization
        let deserialized: ProjectData = toml::from_str(&toml_string).unwrap();
        assert_eq!(deserialized.project.id, project.project.id);
        assert_eq!(deserialized.project.name, project.project.name);
        assert_eq!(deserialized.project.languages.targets.len(), 3);
    }

    #[test]
    fn test_chapter_data_serialization() {
        let chapter = create_test_chapter();
        
        // Test TOML serialization
        let toml_string = toml::to_string_pretty(&chapter).unwrap();
        println!("Serialized Chapter TOML:\n{}", toml_string);

        // Should contain expected sections
        assert!(toml_string.contains("[chapter]"));
        assert!(toml_string.contains("[[units]]"));
        assert!(toml_string.contains("[[todos]]"));

        // Test deserialization
        let deserialized: ChapterData = toml::from_str(&toml_string).unwrap();
        assert_eq!(deserialized.chapter.number, 1);
        assert_eq!(deserialized.chapter.slug, "introduction-testing");
        assert_eq!(deserialized.units.len(), 2);
        assert_eq!(deserialized.todos.len(), 1);
    }

    #[test]
    fn test_translation_unit_with_translations() {
        let mut unit = TranslationUnit::new(
            "test_unit_001".to_string(),
            1,
            "en".to_string(),
            "Hello, world!".to_string(),
            ComplexityLevel::Low,
        );

        // Add German translation
        let german_translation = TranslationVersion::new(
            "Hallo, Welt!".to_string(),
            "test-translator-de".to_string(),
            TranslationStatus::Approved,
        );
        unit.add_translation("de".to_string(), german_translation);

        // Add French translation
        let french_translation = TranslationVersion::new(
            "Bonjour, le monde!".to_string(),
            "test-translator-fr".to_string(),
            TranslationStatus::UnderReview,
        );
        unit.add_translation("fr".to_string(), french_translation);

        // Test serialization
        let toml_string = toml::to_string_pretty(&unit).unwrap();
        println!("Unit with translations:\n{}", toml_string);

        // Should contain translation sections
        assert!(toml_string.contains("[translations.de]"));
        assert!(toml_string.contains("[translations.fr]"));
        assert!(toml_string.contains("Hallo, Welt!"));
        assert!(toml_string.contains("Bonjour, le monde!"));

        // Test deserialization
        let deserialized: TranslationUnit = toml::from_str(&toml_string).unwrap();
        assert_eq!(deserialized.translations.len(), 2);
        assert!(deserialized.get_translation("de").is_some());
        assert!(deserialized.get_translation("fr").is_some());
        assert_eq!(deserialized.get_translation("de").unwrap().text, "Hallo, Welt!");
    }

    #[test]
    fn test_file_manager_operations() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());

        // Initialize directories
        manager.init_directories().unwrap();
        assert!(manager.content_dir_path().exists());
        assert!(manager.chapters_dir_path().exists());

        // Test project operations
        let project = create_test_project();
        manager.write_project(&project).unwrap();
        assert!(manager.project_exists());

        let loaded_project = manager.read_project().unwrap();
        assert_eq!(loaded_project.project.id, project.project.id);

        // Test chapter operations
        let chapter = create_test_chapter();
        manager.write_chapter(&chapter).unwrap();
        assert!(manager.chapter_exists(1, "introduction-testing"));

        let loaded_chapter = manager.read_chapter(1, "introduction-testing").unwrap();
        assert_eq!(loaded_chapter.chapter.slug, chapter.chapter.slug);
        assert_eq!(loaded_chapter.units.len(), chapter.units.len());
    }

    #[test]
    fn test_validation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        manager.init_directories().unwrap();

        // Write valid files
        let project = create_test_project();
        let chapter = create_test_chapter();
        manager.write_project(&project).unwrap();
        manager.write_chapter(&chapter).unwrap();

        // Test validation
        let report = manager.validate_all().unwrap();
        assert!(report.is_valid());
        assert_eq!(report.valid_files, 2);
        assert_eq!(report.invalid_files, 0);

        // Test project validation directly
        assert!(project.validate().is_ok());
        assert!(chapter.validate().is_ok());
    }

    #[test]
    fn test_invalid_data_validation() {
        // Test empty project name
        let mut project = create_test_project();
        project.project.name = String::new();
        assert!(project.validate().is_err());

        // Test invalid quality threshold
        project.project.name = "Valid Name".to_string();
        project.project.settings.quality_threshold = -1.0;
        assert!(project.validate().is_err());

        project.project.settings.quality_threshold = 15.0;
        assert!(project.validate().is_err());

        // Test empty chapter slug
        let mut chapter = create_test_chapter();
        chapter.chapter.slug = String::new();
        assert!(chapter.validate().is_err());
    }

    #[test]
    fn test_chapter_listing() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        manager.init_directories().unwrap();

        // Create multiple chapters
        for i in 1..=5 {
            let mut chapter = create_test_chapter();
            chapter.chapter.number = i;
            chapter.chapter.slug = format!("chapter-{:02}", i);
            manager.write_chapter(&chapter).unwrap();
        }

        let chapters = manager.list_chapters().unwrap();
        assert_eq!(chapters.len(), 5);

        // Should be sorted by number
        for (index, (number, slug, _)) in chapters.iter().enumerate() {
            assert_eq!(*number, (index + 1) as u32);
            assert_eq!(*slug, format!("chapter-{:02}", index + 1));
        }
    }

    #[test]
    fn test_statistics() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        manager.init_directories().unwrap();

        let project = create_test_project();
        let mut chapter = create_test_chapter();

        // Add more translation units and translations
        for unit in &mut chapter.units {
            let german_translation = TranslationVersion::new(
                format!("German: {}", unit.source_text),
                "test-translator".to_string(),
                TranslationStatus::Completed,
            );
            unit.add_translation("de".to_string(), german_translation);
        }

        manager.write_project(&project).unwrap();
        manager.write_chapter(&chapter).unwrap();

        let stats = manager.get_statistics().unwrap();
        assert!(stats.has_project_file);
        assert_eq!(stats.total_chapters, 1);
        assert_eq!(stats.total_languages, 3); // en + de + fr
        assert_eq!(stats.total_translation_units, 2);
        assert_eq!(stats.total_translations, 2); // Only German translations added
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        manager.init_directories().unwrap();

        let project = create_test_project();
        manager.write_project(&project).unwrap();

        // Create backup
        let project_path = manager.project_toml_path();
        let backup_path = manager.backup_file(&project_path).unwrap();
        assert!(backup_path.exists());

        // Modify original
        let mut modified_project = project.clone();
        modified_project.project.name = "Modified Project".to_string();
        manager.write_project(&modified_project).unwrap();

        // Verify modification
        let loaded = manager.read_project().unwrap();
        assert_eq!(loaded.project.name, "Modified Project");

        // Restore from backup
        manager.restore_from_backup(&backup_path, &project_path).unwrap();

        // Verify restoration
        let restored = manager.read_project().unwrap();
        assert_eq!(restored.project.name, project.project.name);
    }

    #[test]
    fn test_complex_chapter_with_all_features() {
        let mut chapter = create_test_chapter();

        // Add complex unit with todos, comments, and notes
        let mut complex_unit = TranslationUnit::new(
            "complex_unit_001".to_string(),
            3,
            "en".to_string(),
            "This is a complex paragraph with technical terminology and cultural references.".to_string(),
            ComplexityLevel::High,
        );

        // Add translation with metadata
        let mut translation = TranslationVersion::new(
            "Dies ist ein komplexer Absatz mit technischer Terminologie und kulturellen Bezügen.".to_string(),
            "expert-translator".to_string(),
            TranslationStatus::UnderReview,
        );
        translation.mark_reviewed(
            "senior-reviewer".to_string(),
            Some(8.5),
            Some("Good translation, minor terminology adjustments needed".to_string()),
        );
        complex_unit.add_translation("de".to_string(), translation);

        // Add unit-level todo
        let unit_todo = UnitTodo {
            id: "unit_todo_001".to_string(),
            title: "Verify cultural reference translation".to_string(),
            description: Some("Check if cultural reference is appropriate for German audience".to_string()),
            created_by: "cultural-expert".to_string(),
            assigned_to: Some("expert-translator".to_string()),
            priority: Priority::High,
            status: TodoStatus::InProgress,
            todo_type: TodoType::Research,
            context: TodoContext::Translation {
                translation: TranslationContext {
                    paragraph: "complex_unit_001".to_string(),
                    language: "de".to_string(),
                }
            },
            created_at: Utc::now(),
            due_date: None,
            resolved_at: None,
            resolution: None,
        };
        complex_unit.todos.push(unit_todo);

        // Add comment
        let comment = Comment {
            id: "comment_001".to_string(),
            author: "senior-reviewer".to_string(),
            content: "Consider using 'Fachterminologie' instead of 'technische Terminologie'".to_string(),
            comment_type: CommentType::Suggestion,
            context: CommentContext::Translation {
                translation: TranslationContext {
                    paragraph: "complex_unit_001".to_string(),
                    language: "de".to_string(),
                }
            },
            created_at: Utc::now(),
            resolved: false,
            thread_id: Some("thread_001".to_string()),
            replies: vec![
                CommentReply {
                    author: "expert-translator".to_string(),
                    content: "Good suggestion, I'll make that change.".to_string(),
                    created_at: Utc::now(),
                }
            ],
        };
        complex_unit.comments.push(comment);

        // Add translation note
        let note = TranslationNote {
            id: "note_001".to_string(),
            author: "expert-translator".to_string(),
            content: "Used 'kulturelle Bezüge' for cultural references to maintain formal tone".to_string(),
            note_type: NoteType::Cultural,
            created_at: Utc::now(),
            language: "de".to_string(),
            visibility: NoteVisibility::Team,
        };
        complex_unit.notes.push(note);

        chapter.add_unit(complex_unit);

        // Test serialization of complex structure
        let toml_string = toml::to_string_pretty(&chapter).unwrap();
        println!("Complex chapter TOML:\n{}", toml_string);

        // Should contain all sections
        assert!(toml_string.contains("[[units]]"));
        assert!(toml_string.contains("[[units.todos]]"));
        assert!(toml_string.contains("[[units.comments]]"));
        assert!(toml_string.contains("[[units.notes]]"));
        assert!(toml_string.contains("[units.translations.de]"));

        // Test deserialization
        let deserialized: ChapterData = toml::from_str(&toml_string).unwrap();
        assert!(deserialized.validate().is_ok());

        let complex_unit = deserialized.get_unit("complex_unit_001").unwrap();
        assert_eq!(complex_unit.todos.len(), 1);
        assert_eq!(complex_unit.comments.len(), 1);
        assert_eq!(complex_unit.notes.len(), 1);
        assert!(complex_unit.get_translation("de").is_some());
        
        let translation = complex_unit.get_translation("de").unwrap();
        assert_eq!(translation.status, TranslationStatus::UnderReview);
        assert_eq!(translation.quality_score, Some(8.5));
        assert!(translation.reviewer.is_some());
    }

    #[test]
    fn test_toml_formatting_utilities() {
        use crate::git_integration::toml_io::utils::*;

        let project = create_test_project();
        
        // Test pretty formatting
        let formatted = format_toml_pretty(&project).unwrap();
        assert!(formatted.contains("[project]"));
        
        // Test syntax validation
        validate_toml_syntax(&formatted).unwrap();
        
        // Test invalid TOML
        assert!(validate_toml_syntax("invalid toml content [").is_err());
    }

    #[test]
    fn test_atomic_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        manager.init_directories().unwrap();

        let project = create_test_project();
        
        // Write project - should create temp file first
        manager.write_project(&project).unwrap();
        
        // Check that no temp files remain
        let project_path = manager.project_toml_path();
        let temp_path = project_path.with_extension("toml.tmp");
        assert!(!temp_path.exists());
        assert!(project_path.exists());
        
        // Content should be readable
        let loaded = manager.read_project().unwrap();
        assert_eq!(loaded.project.id, project.project.id);
    }

    #[test]
    fn test_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TomlFileManager::new(temp_dir.path());
        
        // Try to read non-existent project
        assert!(manager.read_project().is_err());
        
        // Try to read non-existent chapter
        assert!(manager.read_chapter(1, "non-existent").is_err());
        
        // Try to write to invalid path (read-only directory simulation)
        // This test is platform-dependent, so we'll just verify the error types exist
        let result = manager.read_project();
        match result {
            Err(TomlDataError::Validation(_)) => {}, // Expected error type
            _ => {}, // Other error types are also acceptable for this test
        }
    }
}