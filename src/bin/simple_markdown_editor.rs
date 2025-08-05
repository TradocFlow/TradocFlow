use slint::*;

slint::include_modules!();

// Simple Slint component for basic markdown editing
slint::slint! {
    component SimpleMarkdownEditor inherits Window {
        title: "Simple Markdown Editor";
        width: 1000px;
        height: 700px;
        
        in-out property <string> content: "# Welcome to Simple Markdown Editor\n\nStart typing your markdown here...\n\n## Features\n\n- Live markdown editing\n- File save/load\n- Basic formatting\n\n### Getting Started\n\n1. Type your markdown in the editor\n2. Use File menu to save/load\n3. Content is saved automatically\n\nEnjoy writing!";
        
        callback file-save();
        callback file-open();
        callback content-changed(string);
        
        VerticalBox {
            // Menu bar
            HorizontalBox {
                height: 40px;
                padding: 8px;
                background: #f0f0f0;
                
                Button {
                    text: "📁 Open";
                    width: 80px;
                    clicked => { file-open(); }
                }
                
                Button {
                    text: "💾 Save";
                    width: 80px;
                    clicked => { file-save(); }
                }
                
                Text {
                    text: "Simple Markdown Editor - Type your markdown below";
                    vertical-alignment: center;
                    color: #666;
                }
            }
            
            // Editor area
            ScrollView {
                TextEdit {
                    text: content;
                    font-family: "Liberation Mono, Consolas, monospace";
                    font-size: 14px;
                    
                    edited(text) => {
                        content = text;
                        content-changed(text);
                    }
                }
            }
            
            // Status bar
            HorizontalBox {
                height: 30px;
                padding: 8px;
                background: #e0e0e0;
                
                Text {
                    text: @tr("Words: {}", content.split(" ").length);
                    font-size: 12px;
                    color: #666;
                }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = SimpleMarkdownEditor::new()?;
    
    // Set up file operations
    let ui_handle = ui.as_weak();
    ui.on_file_save(move || {
        let ui = ui_handle.unwrap();
        let content = ui.get_content().to_string();
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .set_file_name("document.md")
            .save_file()
        {
            match std::fs::write(&path, content) {
                Ok(_) => println!("✅ File saved: {}", path.display()),
                Err(e) => eprintln!("❌ Failed to save file: {}", e),
            }
        }
    });
    
    let ui_handle = ui.as_weak();
    ui.on_file_open(move || {
        let ui = ui_handle.unwrap();
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    ui.set_content(content.into());
                    println!("✅ File loaded: {}", path.display());
                },
                Err(e) => eprintln!("❌ Failed to load file: {}", e),
            }
        }
    });
    
    let ui_handle = ui.as_weak();
    ui.on_content_changed(move |text| {
        println!("📝 Content updated ({} characters)", text.len());
    });
    
    println!("🚀 Simple Markdown Editor started!");
    println!("💡 Use the Open/Save buttons to work with files");
    println!("💡 Start typing in the text area to edit markdown");
    
    ui.run()
}