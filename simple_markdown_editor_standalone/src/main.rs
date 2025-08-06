use slint::*;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    
    // Set up file save functionality
    let ui_handle = ui.as_weak();
    ui.on_file_save(move || {
        let ui = ui_handle.unwrap();
        let content = ui.get_content().to_string();
        
        // Use file dialog to save
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .set_file_name("document.md")
            .save_file()
        {
            match std::fs::write(&path, content) {
                Ok(_) => {
                    println!("✅ File saved successfully: {}", path.display());
                },
                Err(e) => {
                    eprintln!("❌ Failed to save file: {}", e);
                }
            }
        }
    });
    
    // Set up file open functionality
    let ui_handle = ui.as_weak();
    ui.on_file_open(move || {
        let ui = ui_handle.unwrap();
        
        // Use file dialog to open
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .add_filter("All Text", &["*"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    ui.set_content(content.into());
                    println!("✅ File loaded successfully: {}", path.display());
                },
                Err(e) => {
                    eprintln!("❌ Failed to load file: {}", e);
                }
            }
        }
    });
    
    // Set up content change tracking
    let _ui_handle = ui.as_weak();
    ui.on_content_changed(move |text| {
        let char_count = text.len();
        let word_count = text.split_whitespace().count();
        println!("📝 Content updated: {} words, {} characters", word_count, char_count);
    });
    
    println!("🚀 Simple Markdown Editor started!");
    println!("💡 Features:");
    println!("   - Click 'Open' to load a markdown file");
    println!("   - Click 'Save' to save your work");
    println!("   - Type directly in the editor to create markdown");
    println!("   - Word count is shown in the status bar");
    println!("   - Window can be resized as needed");
    
    ui.run()
}