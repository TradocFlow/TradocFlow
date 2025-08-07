use env_logger;
use tradocflow_core::gui::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("Starting Tradocument Reviewer GUI...");
    
    // Create and run the application
    match App::new().await {
        Ok(app) => {
            println!("✅ GUI Application initialized successfully");
            println!("✅ Slint UI components loaded");
            println!("✅ Thread-safe callbacks configured");
            println!("✅ Text editing functionality enabled");
            println!("✅ Keyboard shortcuts active");
            println!("💡 Press Ctrl+M to toggle between Markdown and Presentation modes");
            println!("💡 Press Ctrl+1/2/3 to switch layouts");
            println!("💡 Use toolbar buttons or Ctrl+B/I/U for formatting");
            
            // Initialize the application (load last project, etc.)
            if let Err(e) = app.initialize().await {
                eprintln!("⚠️ Warning: Failed to initialize application state: {}", e);
            }
            
            // Run the application - this will block until the window is closed
            app.run()?;
            
            println!("GUI Application closed successfully");
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize GUI application: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
