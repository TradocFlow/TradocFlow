use slint::*;

slint::include_modules!();

// Simple Slint component for basic markdown editing
slint::slint! {
    export component SimpleMarkdownEditor inherits Window {
        title: "Simple Markdown Editor";
        width: 1000px;
        height: 700px;
        
        in-out property <string> content: "# Welcome to Simple Markdown Editor\n\nStart typing your markdown here...\n\n## Features\n\n- Live markdown editing\n- File save/load\n- Basic formatting\n\n### Getting Started\n\n1. Type your markdown in the editor\n2. Use File menu to save/load\n3. Content is saved automatically\n\nEnjoy writing!\n\n<!-- TODO: Future UI enhancements -->\n<!-- - Add toolbar with formatting buttons -->\n<!-- - Add split view with live preview -->\n<!-- - Add syntax highlighting -->\n<!-- - Add word/line count display -->";
        
        callback file-save();
        callback file-open();
        callback content-changed(string);
        
        Rectangle {
            // Menu bar
            Rectangle {
                height: 40px;
                background: #f8f9fa;
                border-width: 1px;
                border-color: #dee2e6;
                
                // Open button
                TouchArea {
                    width: 80px;
                    height: 30px;
                    x: 10px;
                    y: 5px;
                    
                    clicked => { file-open(); }
                    
                    Rectangle {
                        background: parent.has-hover ? #e8e8e8 : #f0f0f0;
                        border-width: 1px;
                        border-color: #bbb;
                        border-radius: 4px;
                        
                        Text {
                            text: "📁 Open";
                            color: #333;
                            horizontal-alignment: center;
                            vertical-alignment: center;
                            font-size: 13px;
                        }
                    }
                }
                
                // Save button
                TouchArea {
                    width: 80px;
                    height: 30px;
                    x: 100px;
                    y: 5px;
                    
                    clicked => { file-save(); }
                    
                    Rectangle {
                        background: parent.has-hover ? #e8e8e8 : #f0f0f0;
                        border-width: 1px;
                        border-color: #bbb;
                        border-radius: 4px;
                        
                        Text {
                            text: "💾 Save";
                            color: #333;
                            horizontal-alignment: center;
                            vertical-alignment: center;
                            font-size: 13px;
                        }
                    }
                }
                
                Text {
                    text: "Simple Markdown Editor - Type your markdown below";
                    x: 200px;
                    y: 12px;
                    color: #666;
                    font-size: 13px;
                }
            }
            
            // Editor area - actual text editor
            Rectangle {
                y: 40px;
                height: parent.height - 70px;
                background: white;
                border-width: 1px;
                border-color: #ccc;
                
                // Scrollable text editor
                Flickable {
                    width: parent.width;
                    height: parent.height;
                    viewport-width: parent.width - 20px;
                    viewport-height: text-editor.preferred-height;
                    
                    text-editor := TextInput {
                        text: content;
                        font-family: "Liberation Mono, Consolas, monospace";
                        font-size: 14px;
                        x: 10px;
                        y: 10px;
                        width: parent.viewport-width - 20px;
                        color: black;
                        single-line: false;
                        wrap: word-wrap;
                        
                        edited => {
                            content = self.text;
                            content-changed(self.text);
                        }
                    }
                }
            }
            
            // Status bar
            Rectangle {
                y: parent.height - 30px;
                height: 30px;
                background: #e0e0e0;
                border-width: 1px;
                border-color: #ccc;
                
                Text {
                    text: "Simple Markdown Editor - Ready to edit";
                    x: 8px;
                    y: 8px;
                    font-size: 12px;
                    color: #666;
                }
                
                Text {
                    text: "Ready";
                    x: parent.width - 80px;
                    y: 8px;
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
    
    ui.on_content_changed(move |text| {
        println!("📝 Content updated ({} characters)", text.len());
    });
    
    println!("🚀 Simple Markdown Editor started!");
    println!("💡 Use the Open/Save buttons to work with files");
    println!("💡 Start typing in the text area to edit markdown");
    
    // TODO: Future enhancements
    // - Add syntax highlighting for markdown
    // - Add live preview panel 
    // - Add word count and line count in status bar
    // - Add recent files menu
    // - Add find/replace functionality
    // - Add markdown toolbar with formatting buttons
    // - Add auto-save functionality
    // - Add split view (editor + preview)
    
    ui.run()
}