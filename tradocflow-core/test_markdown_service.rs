use std::path::Path;

// Add the path to the source
fn main() {
    // Add the src directory to the module path
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&manifest_dir).join("src");
    println!("cargo:rustc-link-search={}", src_path.display());
}

#[path = "src/services/markdown_service.rs"]
mod markdown_service;

use markdown_service::*;

fn main() {
    println!("Testing MarkdownService...");
    
    let service = MarkdownService::new();
    
    // Test basic rendering
    let markdown = "# Hello World\n\nThis is a **bold** text.";
    match service.render_to_html(markdown) {
        Ok(html) => {
            println!("✓ Basic rendering works");
            println!("HTML: {}", html);
        }
        Err(e) => {
            println!("✗ Basic rendering failed: {}", e);
            return;
        }
    }
    
    // Test formatting
    match service.apply_formatting("text", FormatType::Bold) {
        Ok(result) => {
            println!("✓ Bold formatting works: {}", result);
        }
        Err(e) => {
            println!("✗ Bold formatting failed: {}", e);
        }
    }
    
    // Test validation
    let invalid_markdown = "#######Invalid Heading";
    match service.validate_syntax(invalid_markdown) {
        Ok(errors) => {
            if !errors.is_empty() {
                println!("✓ Validation works, found {} errors", errors.len());
                for error in errors {
                    println!("  - {}", error.message);
                }
            } else {
                println!("✗ Validation should have found errors");
            }
        }
        Err(e) => {
            println!("✗ Validation failed: {}", e);
        }
    }
    
    println!("MarkdownService test completed!");
}