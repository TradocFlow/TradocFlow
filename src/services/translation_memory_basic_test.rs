#[cfg(test)]
mod basic_tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;
    use crate::models::translation_models::{
        TranslationUnit, LanguagePair
    };

    async fn create_test_service() -> (TranslationMemoryService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = TranslationMemoryService::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        (service, temp_dir)
    }

    fn create_test_translation_unit(
        source_text: &str,
        target_text: &str,
        confidence: f32,
    ) -> TranslationUnit {
        TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            source_text.to_string(),
            "es".to_string(),
            target_text.to_string(),
            confidence,
            None,
        ).unwrap()
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        // Test creating translation memory
        let result = service.create_translation_memory(project_id).await;
        assert!(result.is_ok());
        
        // Test adding a translation unit
        let unit = create_test_translation_unit("Hello", "Hola", 0.9);
        let result = service.add_translation_unit(unit).await;
        assert!(result.is_ok());
        
        // Test similarity calculation
        let similarity = service.calculate_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
        
        let similarity = service.calculate_similarity("hello", "goodbye");
        assert_eq!(similarity, 0.0);
    }
}