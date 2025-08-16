use slint::*;

// Import the GUI framework components
use tradocflow_core::gui::App;
use tradocflow_core::MainWindow;

/// Hybrid Markdown Editor that combines the functional editing from simple editor
/// with the advanced UI features from the full GUI application
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Hybrid Markdown Editor...");
    
    // Try to initialize the advanced GUI app for backend functionality
    match App::new().await {
        Ok(app) => {
            println!("✅ Advanced GUI Application initialized successfully");
            
            // Initialize the application state
            if let Err(e) = app.initialize().await {
                eprintln!("⚠️ Warning: Failed to initialize application state: {}", e);
            }
            
            println!("💡 Hybrid Editor Features:");
            println!("   • Advanced UI with full functionality");
            println!("   • Working text editor with cursor support");
            println!("   • Toolbar with formatting buttons");
            println!("   • Horizontal/Vertical split views (Ctrl+2/3)");
            println!("   • Mode switching (Ctrl+M)");
            println!("   • All file operations (Ctrl+N/O/S)");
            println!("   • Export functionality (Ctrl+E)");
            
            // Run the application - this will block until the window is closed
            app.run()?;
            
            println!("Advanced Hybrid Markdown Editor closed successfully");
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize advanced GUI: {}", e);
            
            // Fallback to simple editor approach if advanced app fails
            println!("🔄 Falling back to simple editor implementation...");
            run_simple_fallback().await?;
        }
    }
    
    Ok(())
}


/// Fallback implementation using simple editor approach
async fn run_simple_fallback() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Initializing fallback simple editor...");
    let ui = MainWindow::new()?;
    
    // Simple but functional setup
    ui.set_document_content("# Fallback Markdown Editor\n\nThe advanced GUI failed to initialize, but you still have a working editor!\n\nStart typing your markdown here...".into());
    ui.set_current_mode("markdown".into());
    ui.set_current_layout("single".into());
    ui.set_status_message("Ready - Fallback Mode".into());
    ui.set_status_type("warning".into());
    
    // Basic file operations
    let ui_handle = ui.as_weak();
    ui.on_file_new(move || {
        let ui = ui_handle.unwrap();
        ui.set_document_content("# New Document\n\nStart typing your markdown here...".into());
        ui.set_status_message("New document created".into());
        ui.set_status_type("success".into());
        println!("📄 New document created");
    });
    
    let ui_handle = ui.as_weak();
    ui.on_file_save(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_message("File save requested".into());
        ui.set_status_type("info".into());
        println!("💾 Save file requested");
    });
    
    println!("🔄 Running in fallback mode");
    ui.run()?;
    Ok(())
}