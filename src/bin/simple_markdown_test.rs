use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 TradocFlow Markdown Editor Demo");
    println!("===================================\n");
    
    // Sample markdown content
    let sample_markdown = r#"# Sample Document

This is a **sample document** to demonstrate the live preview functionality of the markdown editor.

## Features

- **Live Preview**: See your markdown rendered in real-time
- **Inline Editing**: Click on any element to edit it directly
- **Syntax Highlighting**: Full support for markdown syntax
- **Word Count**: Track your document statistics

### Code Example

```rust
fn main() {
    println!("Hello, markdown world!");
}
```

### Task List

- [x] Implement basic markdown rendering
- [x] Add live preview functionality
- [ ] Add inline editing capabilities
- [ ] Implement syntax highlighting

> **Note**: This is a blockquote to show how different elements are rendered.

## Tables

| Feature | Status | Priority |
|---------|--------|----------|
| Live Preview | ✅ Complete | High |
| Inline Editing | 🚧 In Progress | High |
| Syntax Highlighting | 📋 Planned | Medium |

---

*This document was created with the TradocFlow markdown editor.*"#;
    
    println!("📝 Sample Markdown Content:");
    println!("{}", "-".repeat(50));
    println!("{}", sample_markdown);
    println!("{}", "-".repeat(50));
    
    // Show basic statistics
    let word_count = sample_markdown.split_whitespace().count();
    let line_count = sample_markdown.lines().count();
    let heading_count = sample_markdown.lines().filter(|line| line.starts_with('#')).count();
    let list_item_count = sample_markdown.lines().filter(|line| line.trim_start().starts_with('-')).count();
    
    println!("\n📊 Document Statistics:");
    println!("  Words: {}", word_count);
    println!("  Lines: {}", line_count);
    println!("  Headings: {}", heading_count);
    println!("  List Items: {}", list_item_count);
    
    // Demo of inline editing simulation
    println!("\n🎯 Inline Editing Demo");
    println!("Simulating element editing...");
    
    let mut current_content = sample_markdown.to_string();
    
    // Simulate editing the first heading
    let lines: Vec<&str> = current_content.lines().collect();
    let mut new_lines = lines.clone();
    
    if !new_lines.is_empty() && new_lines[0].starts_with('#') {
        new_lines[0] = "# Updated Sample Document";
        println!("✅ Updated heading: '{}'", new_lines[0]);
    }
    
    // Simulate adding a new paragraph
    new_lines.insert(2, "\nThis paragraph was added through inline editing!");
    println!("✅ Added new paragraph at line 3");
    
    let updated_content = new_lines.join("\n");
    
    println!("\n📝 Updated Content Preview:");
    println!("{}", "-".repeat(30));
    for (i, line) in updated_content.lines().take(10).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }
    println!("... (truncated)");
    println!("{}", "-".repeat(30));
    
    // Interactive demo
    println!("\n🎮 Interactive Demo");
    println!("Enter markdown text to see basic processing (or 'quit' to exit):");
    
    loop {
        print!("markdown> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "quit" || input == "exit" {
            break;
        }
        
        if input.is_empty() {
            continue;
        }
        
        // Basic markdown processing simulation
        let processed = process_markdown_line(input);
        println!("Processed: {}", processed);
        
        // Show element type
        let element_type = detect_element_type(input);
        println!("Element Type: {}", element_type);
        
        // Show word count
        let words = input.split_whitespace().count();
        println!("Words: {}", words);
        println!();
    }
    
    println!("\n👋 Thanks for trying the TradocFlow Markdown Editor!");
    Ok(())
}

fn process_markdown_line(input: &str) -> String {
    // Simple markdown-to-HTML-like processing
    let mut result = input.to_string();
    
    // Bold text
    while let (Some(start), Some(end)) = (result.find("**"), result.rfind("**")) {
        if start != end {
            let before = &result[..start];
            let content = &result[start + 2..end];
            let after = &result[end + 2..];
            result = format!("{}<strong>{}</strong>{}", before, content, after);
        } else {
            break;
        }
    }
    
    // Italic text
    while let (Some(start), Some(end)) = (result.find('*'), result.rfind('*')) {
        if start != end && !result[start..=start].contains("**") {
            let before = &result[..start];
            let content = &result[start + 1..end];
            let after = &result[end + 1..];
            result = format!("{}<em>{}</em>{}", before, content, after);
        } else {
            break;
        }
    }
    
    // Code spans
    while let (Some(start), Some(end)) = (result.find('`'), result.rfind('`')) {
        if start != end {
            let before = &result[..start];
            let content = &result[start + 1..end];
            let after = &result[end + 1..];
            result = format!("{}<code>{}</code>{}", before, content, after);
        } else {
            break;
        }
    }
    
    result
}

fn detect_element_type(input: &str) -> &'static str {
    let trimmed = input.trim();
    
    if trimmed.starts_with("# ") {
        "Heading 1"
    } else if trimmed.starts_with("## ") {
        "Heading 2"
    } else if trimmed.starts_with("### ") {
        "Heading 3"
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        "List Item"
    } else if trimmed.starts_with("- [ ]") {
        "Task Item (Unchecked)"
    } else if trimmed.starts_with("- [x]") {
        "Task Item (Checked)"
    } else if trimmed.starts_with("> ") {
        "Blockquote"
    } else if trimmed.starts_with("```") {
        "Code Block"
    } else if trimmed.starts_with("|") && trimmed.ends_with("|") {
        "Table Row"
    } else if trimmed.is_empty() {
        "Empty Line"
    } else {
        "Paragraph"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_element_detection() {
        assert_eq!(detect_element_type("# Heading"), "Heading 1");
        assert_eq!(detect_element_type("## Heading"), "Heading 2");
        assert_eq!(detect_element_type("- List item"), "List Item");
        assert_eq!(detect_element_type("- [ ] Task"), "Task Item (Unchecked)");
        assert_eq!(detect_element_type("- [x] Done"), "Task Item (Checked)");
        assert_eq!(detect_element_type("> Quote"), "Blockquote");
        assert_eq!(detect_element_type("Regular text"), "Paragraph");
    }
    
    #[test]
    fn test_markdown_processing() {
        assert_eq!(process_markdown_line("**bold**"), "<strong>bold</strong>");
        assert_eq!(process_markdown_line("`code`"), "<code>code</code>");
        assert_eq!(process_markdown_line("**bold** and `code`"), "<strong>bold</strong> and <code>code</code>");
    }
}