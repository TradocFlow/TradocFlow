use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tempfile::TempDir;

use tradocflow_core::services::{TerminologyServiceAdapter, TerminologyHighlightingService, HighlightType};
use tradocflow_translation_memory::Term;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Terminology Highlighting Demo");
    println!("================================");

    // Create temporary directory for demo
    let temp_dir = TempDir::new()?;
    println!("📁 Created temporary project at: {:?}", temp_dir.path());

    // Initialize services
    let terminology_service = Arc::new(TerminologyServiceAdapter::new(temp_dir.path().to_path_buf()).await?);
    let highlighting_service = TerminologyHighlightingService::new(terminology_service.clone());
    
    let project_id = Uuid::new_v4();
    println!("🆔 Project ID: {}", project_id);

    // Add some sample terminology
    println!("\n📚 Adding sample terminology...");
    let terms = vec![
        Term {
            id: Uuid::new_v4(),
            term: "API".to_string(),
            definition: Some("Application Programming Interface".to_string()),
            do_not_translate: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Term {
            id: Uuid::new_v4(),
            term: "JSON".to_string(),
            definition: Some("JavaScript Object Notation".to_string()),
            do_not_translate: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Term {
            id: Uuid::new_v4(),
            term: "database".to_string(),
            definition: Some("A structured collection of data".to_string()),
            do_not_translate: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Term {
            id: Uuid::new_v4(),
            term: "user interface".to_string(),
            definition: Some("The means by which a user interacts with a system".to_string()),
            do_not_translate: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    for term in &terms {
        // TODO: Implement term addition in TerminologyServiceAdapter
        println!("  ✅ Would add: {} ({})", term.term, 
                if term.do_not_translate { "do not translate" } else { "translatable" });
    }

    // Test text with terminology
    let test_text = "The API uses JSON format to communicate with the database through the user interface. The system provides a RESTful API for data access.";
    println!("\n📝 Test text:");
    println!("\"{}\"", test_text);

    // Analyze text for highlights
    println!("\n🎯 Analyzing text for terminology highlights...");
    let highlights = highlighting_service
        .highlight_terms_in_text(test_text, project_id, "en")
        .await?;

    println!("Found {} highlighted terms:", highlights.len());
    for highlight in &highlights {
        let highlight_type_str = match highlight.highlight_type {
            HighlightType::DoNotTranslate => "🚫 DO NOT TRANSLATE",
            HighlightType::Inconsistent => "⚠️  INCONSISTENT",
            HighlightType::Suggestion => "💡 SUGGESTION",
            HighlightType::Validated => "✅ VALIDATED",
        };
        
        println!("  {} \"{}\" at positions {}-{}", 
                highlight_type_str, 
                highlight.term,
                highlight.start_position,
                highlight.end_position);
        
        if let Some(definition) = &highlight.definition {
            println!("     Definition: {}", definition);
        }
    }

    // Generate terminology suggestions
    println!("\n💡 Generating terminology suggestions...");
    let suggestion_text = "The application programming interface uses javascript object notation for data exchange.";
    println!("Suggestion text: \"{}\"", suggestion_text);
    
    let suggestions = highlighting_service
        .generate_terminology_suggestions(suggestion_text, project_id, "en")
        .await?;

    if suggestions.is_empty() {
        println!("No suggestions found.");
    } else {
        println!("Found {} suggestions:", suggestions.len());
        for suggestion in &suggestions {
            println!("  💡 \"{}\" → \"{}\" (confidence: {:.1}%)",
                    suggestion.original_text,
                    suggestion.suggested_term,
                    suggestion.confidence * 100.0);
            println!("     Reason: {}", suggestion.reason);
        }
    }

    // Test consistency checking
    println!("\n🔍 Testing consistency across languages...");
    let mut texts = std::collections::HashMap::new();
    texts.insert("en".to_string(), "The API uses JSON format.".to_string());
    texts.insert("de".to_string(), "Die API verwendet JSON Format.".to_string());
    texts.insert("fr".to_string(), "L'interface API utilise le format JSON.".to_string()); // Inconsistent

    let consistency_results = highlighting_service
        .check_consistency_across_languages(texts, project_id)
        .await?;

    if consistency_results.is_empty() {
        println!("No consistency issues found.");
    } else {
        println!("Found {} consistency issues:", consistency_results.len());
        for result in &consistency_results {
            println!("  ⚠️  Term: \"{}\"", result.term);
            for inconsistency in &result.inconsistencies {
                println!("     Language: {} - Expected: \"{}\", Found: {:?}",
                        inconsistency.language,
                        inconsistency.expected_term,
                        inconsistency.found_terms);
            }
        }
    }

    // Test real-time highlighting update
    println!("\n⚡ Testing real-time highlighting updates...");
    let updated_text = "The API uses JSON format to communicate with the database. New API endpoints were added.";
    let change_start = 60; // Around "New API"
    let change_end = 80;
    
    let updated_highlights = highlighting_service
        .update_highlighting_for_text_change(updated_text, change_start, change_end, project_id, "en")
        .await?;

    println!("Updated highlights in changed region:");
    for highlight in &updated_highlights {
        println!("  {} \"{}\" at positions {}-{}",
                match highlight.highlight_type {
                    HighlightType::DoNotTranslate => "🚫",
                    HighlightType::Inconsistent => "⚠️",
                    HighlightType::Suggestion => "💡",
                    HighlightType::Validated => "✅",
                },
                highlight.term,
                highlight.start_position,
                highlight.end_position);
    }

    println!("\n🎉 Demo completed successfully!");
    println!("The terminology highlighting system provides:");
    println!("  ✅ Real-time terminology detection");
    println!("  ✅ Visual highlighting for non-translatable terms");
    println!("  ✅ Terminology consistency checking across languages");
    println!("  ✅ Terminology suggestion system for translators");
    println!("  ✅ Efficient caching and performance optimization");

    Ok(())
}