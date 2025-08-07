use tradocflow_core::gui::MarkdownEditorBridge;
use anyhow::Result;
use std::io::{self, Write};
use slint::Model;

fn main() -> Result<()> {
    println!("🚀 TradocFlow Markdown Editor Demo");
    println!("===================================\n");
    
    // Initialize the markdown editor bridge
    let bridge = MarkdownEditorBridge::new();
    
    // Initialize with sample content
    bridge.initialize_with_sample()?;
    
    // Display current content
    println!("📝 Current Markdown Content:");
    println!("{}", "-".repeat(50));
    println!("{}", bridge.get_content()?);
    println!("{}", "-".repeat(50));
    
    // Render to HTML
    println!("\n🌐 Rendered HTML Preview:");
    println!("{}", "-".repeat(50));
    let html = bridge.export_html()?;
    println!("{}", html);
    println!("{}", "-".repeat(50));
    
    // Show statistics
    println!("\n📊 Document Statistics:");
    let stats = bridge.get_statistics()?;
    for (key, value) in stats {
        println!("  {}: {}", key, value);
    }
    
    // Interactive mode
    println!("\n🎯 Interactive Mode");
    println!("Type 'help' for commands, 'quit' to exit\n");
    
    loop {
        print!("markdown> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        match input {
            "help" => show_help(),
            "quit" | "exit" => break,
            "render" => {
                let content = bridge.get_content()?;
                let rendered = bridge.render_markdown(&content)?;
                println!("📄 Rendered {} elements with {} words", 
                    rendered.elements.row_count(), rendered.word_count);
            },
            "stats" => {
                let stats = bridge.get_statistics()?;
                println!("📊 Statistics:");
                for (key, value) in stats {
                    println!("  {}: {}", key, value);
                }
            },
            "export" => {
                let html = bridge.export_html()?;
                println!("🌐 HTML Export ({} characters):", html.len());
                println!("{}", html);
            },
            "sample" => {
                bridge.initialize_with_sample()?;
                println!("✅ Loaded sample content");
            },
            "validate" => {
                let content = bridge.get_content()?;
                let warnings = bridge.validate_markdown(&content)?;
                if warnings.is_empty() {
                    println!("✅ Markdown is valid!");
                } else {
                    println!("⚠️  Validation warnings:");
                    for warning in warnings {
                        println!("  - {}", warning);
                    }
                }
            },
            input if input.starts_with("edit ") => {
                let parts: Vec<&str> = input.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    let element_id = parts[1];
                    let new_content = parts[2];
                    match bridge.handle_element_edit(element_id, new_content) {
                        Ok(_) => println!("✅ Element '{}' updated", element_id),
                        Err(e) => println!("❌ Error updating element: {}", e),
                    }
                } else {
                    println!("Usage: edit <element_id> <new_content>");
                }
            },
            input if input.starts_with("set ") => {
                let content = &input[4..];
                match bridge.update_content(content) {
                    Ok(_) => println!("✅ Content updated"),
                    Err(e) => println!("❌ Error updating content: {}", e),
                }
            },
            "show" => {
                let content = bridge.get_content()?;
                println!("📝 Current content:");
                println!("{}", content);
            },
            _ if !input.is_empty() => {
                println!("❓ Unknown command: '{}'. Type 'help' for available commands.", input);
            },
            _ => {} // Empty input, do nothing
        }
    }
    
    println!("\n👋 Thanks for using TradocFlow Markdown Editor!");
    Ok(())
}

fn show_help() {
    println!("📚 Available Commands:");
    println!("  help              - Show this help message");
    println!("  render            - Render current markdown to elements");
    println!("  stats             - Show document statistics");
    println!("  export            - Export to HTML");
    println!("  sample            - Load sample content");
    println!("  validate          - Validate markdown syntax");
    println!("  edit <id> <text>  - Edit specific element (e.g., 'edit heading1-0 New Title')");
    println!("  set <content>     - Set entire markdown content");
    println!("  show              - Show current markdown content");
    println!("  quit/exit         - Exit the demo");
    println!();
    println!("💡 Example element IDs:");
    println!("  heading1-0        - First H1 heading (line 0)");
    println!("  paragraph-2       - Paragraph on line 2");
    println!("  list_item-5       - List item on line 5");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_demo_initialization() {
        let bridge = MarkdownEditorBridge::new();
        assert!(bridge.initialize_with_sample().is_ok());
        assert!(!bridge.get_content().unwrap().is_empty());
    }
    
    #[test]
    fn test_demo_operations() {
        let bridge = MarkdownEditorBridge::new();
        bridge.initialize_with_sample().unwrap();
        
        // Test HTML export
        assert!(bridge.export_html().is_ok());
        
        // Test statistics
        let stats = bridge.get_statistics().unwrap();
        assert!(!stats.is_empty());
        
        // Test element editing
        assert!(bridge.handle_element_edit("heading1-0", "New Title").is_ok());
    }
}