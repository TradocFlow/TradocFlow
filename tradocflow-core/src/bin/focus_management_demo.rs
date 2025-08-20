use std::time::Duration;
use tokio::time::sleep;
use tradocflow_core::gui::main_window_focus_bridge::MainWindowFocusBridge;

/// Demo application showing cursor blinking and focus management
/// across different editor layouts
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Focus Management Demo");
    println!("This demo shows cursor blinking and focus switching across editor panes");
    
    let mut focus_bridge = MainWindowFocusBridge::new();
    
    // Demo 1: Single editor layout
    println!("\n📝 Demo 1: Single Editor Layout");
    focus_bridge.initialize_layout("single")?;
    let (active_editor, active_pane) = focus_bridge.get_active_editor_info()?;
    println!("Active editor: {}, Active pane: {}", active_editor, active_pane);
    
    // Enable cursor blinking
    focus_bridge.set_cursor_blink_enabled(true)?;
    println!("✨ Cursor blinking enabled");
    
    // Simulate cursor blinking for 3 seconds
    for i in 0..6 {
        let cursor_states = focus_bridge.tick_cursor_blink()?;
        let has_cursor = cursor_states.get("single-editor").copied().unwrap_or(false);
        println!("Blink {}: Cursor visible: {}", i + 1, has_cursor);
        sleep(Duration::from_millis(500)).await;
    }
    
    // Demo 2: Horizontal split layout
    println!("\n📱 Demo 2: Horizontal Split Layout");
    focus_bridge.handle_layout_change("horizontal")?;
    
    // Test focus switching
    println!("🎯 Testing focus switching...");
    
    // Focus left editor first
    let focus_states = focus_bridge.request_editor_focus("left-editor", "left-pane")?;
    println!("Left editor focus: {}, Right editor focus: {}", 
        focus_states.get("left-editor").copied().unwrap_or(false),
        focus_states.get("right-editor").copied().unwrap_or(false)
    );
    
    sleep(Duration::from_millis(1000)).await;
    
    // Tab to next editor
    let (active_editor, focus_states) = focus_bridge.tab_to_next_editor()?;
    println!("After Tab - Active: {}", active_editor);
    println!("Left editor focus: {}, Right editor focus: {}", 
        focus_states.get("left-editor").copied().unwrap_or(false),
        focus_states.get("right-editor").copied().unwrap_or(false)
    );
    
    sleep(Duration::from_millis(1000)).await;
    
    // Tab back to previous editor
    let (active_editor, focus_states) = focus_bridge.tab_to_previous_editor()?;
    println!("After Shift+Tab - Active: {}", active_editor);
    println!("Left editor focus: {}, Right editor focus: {}", 
        focus_states.get("left-editor").copied().unwrap_or(false),
        focus_states.get("right-editor").copied().unwrap_or(false)
    );
    
    // Demo 3: Grid 2x2 layout
    println!("\n🔲 Demo 3: Grid 2x2 Layout");
    focus_bridge.handle_layout_change("grid_2x2")?;
    
    // Test keyboard shortcuts (Alt+1 through Alt+4)
    println!("⌨️  Testing keyboard shortcuts (Alt+1 to Alt+4)...");
    
    for i in 1..=4 {
        let key = i.to_string();
        if let Some((active_editor, focus_states)) = focus_bridge.handle_keyboard_shortcut(&key, false, false, true)? {
            println!("Alt+{} - Active: {}", i, active_editor);
            
            // Show focus states for all panes
            for pane_num in 1..=4 {
                let editor_id = format!("pane-{}-editor", pane_num);
                let has_focus = focus_states.get(&editor_id).copied().unwrap_or(false);
                println!("  Pane {} focus: {}", pane_num, has_focus);
            }
        }
        
        sleep(Duration::from_millis(1000)).await;
    }
    
    // Demo 4: Cursor position and selection updates
    println!("\n📍 Demo 4: Cursor Position and Selection Updates");
    
    // Update cursor position
    focus_bridge.update_cursor_position("pane-1-editor", 42)?;
    println!("Updated cursor position for pane-1-editor to 42");
    
    // Update text selection
    focus_bridge.update_selection("pane-1-editor", 10, 25)?;
    println!("Updated selection for pane-1-editor: 10-25");
    
    // Demo 5: Cursor blinking with selection
    println!("\n⚡ Demo 5: Cursor Blinking with Text Selection");
    println!("When text is selected, cursor should remain visible");
    
    for i in 0..4 {
        let cursor_states = focus_bridge.tick_cursor_blink()?;
        let has_cursor = cursor_states.get("pane-1-editor").copied().unwrap_or(false);
        println!("Blink {}: Cursor visible (with selection): {}", i + 1, has_cursor);
        sleep(Duration::from_millis(500)).await;
    }
    
    // Clear selection and test normal blinking
    focus_bridge.update_selection("pane-1-editor", 25, 25)?; // No selection
    println!("\n🔄 Cleared selection, testing normal cursor blinking...");
    
    for i in 0..4 {
        let cursor_states = focus_bridge.tick_cursor_blink()?;
        let has_cursor = cursor_states.get("pane-1-editor").copied().unwrap_or(false);
        println!("Blink {}: Cursor visible (no selection): {}", i + 1, has_cursor);
        sleep(Duration::from_millis(500)).await;
    }
    
    // Demo 6: Debug information
    println!("\n🔍 Demo 6: Debug Information");
    let debug_info = focus_bridge.get_debug_info();
    println!("Debug info: {}", debug_info);
    
    // Show all current focus states
    let all_focus_states = focus_bridge.get_all_focus_states();
    println!("\nCurrent focus states:");
    for (editor_id, has_focus) in all_focus_states {
        println!("  {}: {}", editor_id, has_focus);
    }
    
    println!("\n✅ Focus Management Demo completed successfully!");
    println!("Key features demonstrated:");
    println!("  • Cursor blinking in active editor only");
    println!("  • Focus switching between panes");
    println!("  • Tab navigation (Ctrl+Tab, Ctrl+Shift+Tab)");
    println!("  • Keyboard shortcuts (Alt+1-4)");
    println!("  • Cursor visibility with text selection");
    println!("  • Multi-layout support (single, horizontal, grid)");
    
    Ok(())
}