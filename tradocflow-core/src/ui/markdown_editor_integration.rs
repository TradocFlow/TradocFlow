use crate::services::markdown_service::{MarkdownService, RenderedMarkdown, MarkdownElement as ServiceMarkdownElement};
use anyhow::Result;
use slint::{ComponentHandle, ModelRc, VecModel, SharedString};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Slint types that would be generated - we'll define them here for now
#[derive(Clone, Debug)]
pub struct MarkdownElement {
    pub element_type: SharedString,
    pub content: SharedString,
    pub start_line: i32,
    pub start_col: i32,
    pub end_line: i32,
    pub end_col: i32,
    pub editable: bool,
    pub element_id: SharedString,
}

#[derive(Clone, Debug)]
pub struct RenderedContent {
    pub html: SharedString,
    pub elements: ModelRc<slint::VecModel<MarkdownElement>>,
    pub word_count: i32,
    pub heading_count: i32,
}

// Mock UI component for compilation
pub struct MarkdownEditorWithPreview;

// Convert service MarkdownElement to Slint MarkdownElement
fn convert_element_to_slint(element: &ServiceMarkdownElement) -> MarkdownElement {
    MarkdownElement {
        element_type: SharedString::from(element.element_type.clone()),
        content: SharedString::from(element.content.clone()),
        start_line: element.position.start_line as i32,
        start_col: element.position.start_col as i32,
        end_line: element.position.end_line as i32,
        end_col: element.position.end_col as i32,
        editable: element.editable,
        element_id: SharedString::from(format!("{}-{}", element.element_type, element.position.start_line)),
    }
}

// Convert RenderedMarkdown to Slint RenderedContent
fn convert_rendered_to_slint(rendered: &RenderedMarkdown) -> RenderedContent {
    let elements: Vec<MarkdownElement> = rendered.elements
        .iter()
        .map(convert_element_to_slint)
        .collect();
    
    RenderedContent {
        html: SharedString::from(rendered.html.clone()),
        elements: ModelRc::new(VecModel::from(elements)),
        word_count: rendered.metadata.word_count as i32,
        heading_count: rendered.metadata.heading_count as i32,
    }
}

pub struct MarkdownEditorState {
    pub service: MarkdownService,
    pub current_content: String,
    pub rendered_content: Option<RenderedMarkdown>,
    pub auto_render: bool,
    pub render_delay_ms: u64,
}

impl Default for MarkdownEditorState {
    fn default() -> Self {
        Self {
            service: MarkdownService::new(),
            current_content: String::new(),
            rendered_content: None,
            auto_render: true,
            render_delay_ms: 300, // 300ms debounce delay
        }
    }
}

pub struct MarkdownEditorIntegration {
    state: Arc<Mutex<MarkdownEditorState>>,
}

impl MarkdownEditorIntegration {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MarkdownEditorState::default())),
        }
    }
    
    /// Initialize the markdown editor with Slint UI
    pub fn initialize_editor(&self, ui: &MarkdownEditorWithPreview) -> Result<()> {
        let state = Arc::clone(&self.state);
        let ui_weak = ui.as_weak();
        
        // Set up content change handler with debounced rendering
        ui.on_content_changed({
            let state = Arc::clone(&state);
            let ui_weak = ui_weak.clone();
            move |new_content| {
                let content = new_content.to_string();
                
                // Update state
                {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.current_content = content.clone();
                }
                
                // Debounced rendering
                let state_clone = Arc::clone(&state);
                let ui_weak_clone = ui_weak.clone();
                thread::spawn(move || {
                    let delay = {
                        let state_guard = state_clone.lock().unwrap();
                        Duration::from_millis(state_guard.render_delay_ms)
                    };
                    
                    thread::sleep(delay);
                    
                    // Check if content hasn't changed during delay
                    let should_render = {
                        let state_guard = state_clone.lock().unwrap();
                        state_guard.current_content == content && state_guard.auto_render
                    };
                    
                    if should_render {
                        if let Err(e) = Self::render_markdown_async(state_clone, ui_weak_clone) {
                            eprintln!("Failed to render markdown: {}", e);
                        }
                    }
                });
            }
        });
        
        // Set up element editing handler
        ui.on_element_edited({
            let state = Arc::clone(&state);
            let ui_weak = ui_weak.clone();
            move |element_id, new_content| {
                let element_id_str = element_id.to_string();
                let new_content_str = new_content.to_string();
                
                if let Err(e) = Self::handle_element_edit(
                    Arc::clone(&state), 
                    ui_weak.clone(), 
                    &element_id_str, 
                    &new_content_str
                ) {
                    eprintln!("Failed to handle element edit: {}", e);
                }
            }
        });
        
        // Set up preview update handler
        ui.on_preview_updated({
            move |rendered_content| {
                // This callback is triggered when the preview is updated
                // Can be used for additional processing or UI updates
                println!("Preview updated with {} elements", rendered_content.elements.row_count());
            }
        });
        
        // Initial render if there's content
        let initial_content = ui.get_content().to_string();
        if !initial_content.is_empty() {
            {
                let mut state_guard = self.state.lock().unwrap();
                state_guard.current_content = initial_content;
            }
            Self::render_markdown_async(Arc::clone(&self.state), ui_weak)?;
        }
        
        Ok(())
    }
    
    /// Render markdown asynchronously and update UI
    fn render_markdown_async(
        state: Arc<Mutex<MarkdownEditorState>>, 
        ui_weak: slint::Weak<MarkdownEditorWithPreview>
    ) -> Result<()> {
        let (content, service) = {
            let state_guard = state.lock().unwrap();
            (state_guard.current_content.clone(), state_guard.service.clone())
        };
        
        // Render markdown
        let rendered = service.parse_to_elements(&content)?;
        
        // Update state
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.rendered_content = Some(rendered.clone());
        }
        
        // Update UI on main thread
        let ui_weak_clone = ui_weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak_clone.upgrade() {
                let slint_rendered = convert_rendered_to_slint(&rendered);
                ui.invoke_preview_updated(slint_rendered);
                
                // Also update the rendered content property
                // This would require additional Slint integration
            }
        })?;
        
        Ok(())
    }
    
    /// Handle inline element editing
    fn handle_element_edit(
        state: Arc<Mutex<MarkdownEditorState>>,
        ui_weak: slint::Weak<MarkdownEditorWithPreview>,
        element_id: &str,
        new_content: &str,
    ) -> Result<()> {
        let updated_markdown = {
            let state_guard = state.lock().unwrap();
            let current_content = &state_guard.current_content;
            
            // Update the markdown content based on element edit
            Self::update_markdown_content(current_content, element_id, new_content)?
        };
        
        // Update UI content
        let ui_weak_clone = ui_weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak_clone.upgrade() {
                ui.set_content(updated_markdown.clone().into());
            }
        })?;
        
        // Trigger re-render
        Self::render_markdown_async(state, ui_weak)?;
        
        Ok(())
    }
    
    /// Update markdown content with inline edits
    fn update_markdown_content(
        current_markdown: &str,
        element_id: &str,
        new_content: &str,
    ) -> Result<String> {
        // Parse element ID to get type and line number
        let parts: Vec<&str> = element_id.split('-').collect();
        if parts.len() < 2 {
            return Ok(current_markdown.to_string());
        }
        
        let element_type = parts[0];
        let line_number: usize = parts[1].parse().unwrap_or(0);
        
        let mut lines: Vec<String> = current_markdown.lines().map(String::from).collect();
        
        if line_number >= lines.len() {
            return Ok(current_markdown.to_string());
        }
        
        // Update the specific line based on element type
        match element_type {
            "heading1" => lines[line_number] = format!("# {}", new_content),
            "heading2" => lines[line_number] = format!("## {}", new_content),
            "heading3" => lines[line_number] = format!("### {}", new_content),
            "heading4" => lines[line_number] = format!("#### {}", new_content),
            "heading5" => lines[line_number] = format!("##### {}", new_content),
            "heading6" => lines[line_number] = format!("###### {}", new_content),
            "paragraph" => lines[line_number] = new_content.to_string(),
            "list_item" => {
                // Preserve list marker
                if lines[line_number].trim_start().starts_with("- ") {
                    lines[line_number] = format!("- {}", new_content);
                } else if lines[line_number].trim_start().starts_with("* ") {
                    lines[line_number] = format!("* {}", new_content);
                } else if lines[line_number].trim_start().starts_with("+ ") {
                    lines[line_number] = format!("+ {}", new_content);
                } else {
                    // Find list number if it's an ordered list
                    if let Some(dot_pos) = lines[line_number].find('.') {
                        let number_part = &lines[line_number][..dot_pos + 1];
                        lines[line_number] = format!("{} {}", number_part, new_content);
                    } else {
                        lines[line_number] = format!("- {}", new_content);
                    }
                }
            }
            "task_item" => {
                if lines[line_number].contains("[x]") {
                    lines[line_number] = format!("- [x] {}", new_content);
                } else {
                    lines[line_number] = format!("- [ ] {}", new_content);
                }
            }
            "blockquote" => lines[line_number] = format!("> {}", new_content),
            _ => lines[line_number] = new_content.to_string(),
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Get current markdown content
    pub fn get_content(&self) -> String {
        let state_guard = self.state.lock().unwrap();
        state_guard.current_content.clone()
    }
    
    /// Set markdown content
    pub fn set_content(&self, content: &str) -> Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        state_guard.current_content = content.to_string();
        Ok(())
    }
    
    /// Enable/disable auto-rendering
    pub fn set_auto_render(&self, enabled: bool) {
        let mut state_guard = self.state.lock().unwrap();
        state_guard.auto_render = enabled;
    }
    
    /// Set render delay for debouncing
    pub fn set_render_delay(&self, delay_ms: u64) {
        let mut state_guard = self.state.lock().unwrap();
        state_guard.render_delay_ms = delay_ms;
    }
    
    /// Manually trigger markdown rendering
    pub fn render_now(&self, ui_weak: slint::Weak<MarkdownEditorWithPreview>) -> Result<()> {
        Self::render_markdown_async(Arc::clone(&self.state), ui_weak)
    }
    
    /// Export rendered HTML
    pub fn export_html(&self) -> Result<String> {
        let state_guard = self.state.lock().unwrap();
        match &state_guard.rendered_content {
            Some(rendered) => Ok(rendered.html.clone()),
            None => {
                // Render if not already rendered
                let html = state_guard.service.render_to_html(&state_guard.current_content)?;
                Ok(html)
            }
        }
    }
    
    /// Get markdown statistics
    pub fn get_statistics(&self) -> HashMap<String, i32> {
        let mut stats = HashMap::new();
        
        let state_guard = self.state.lock().unwrap();
        if let Some(rendered) = &state_guard.rendered_content {
            stats.insert("words".to_string(), rendered.metadata.word_count as i32);
            stats.insert("headings".to_string(), rendered.metadata.heading_count as i32);
            stats.insert("links".to_string(), rendered.metadata.link_count as i32);
            stats.insert("images".to_string(), rendered.metadata.image_count as i32);
            stats.insert("tables".to_string(), rendered.metadata.table_count as i32);
            stats.insert("elements".to_string(), rendered.elements.len() as i32);
        }
        
        stats
    }
}

impl Default for MarkdownEditorIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markdown_editor_integration() {
        let integration = MarkdownEditorIntegration::new();
        
        // Test setting and getting content
        integration.set_content("# Test Heading").unwrap();
        assert_eq!(integration.get_content(), "# Test Heading");
        
        // Test configuration
        integration.set_auto_render(false);
        integration.set_render_delay(500);
    }
    
    #[test]
    fn test_content_update() {
        let result = MarkdownEditorIntegration::update_markdown_content(
            "# Old Heading\n\nSome content",
            "heading1-0",
            "New Heading"
        ).unwrap();
        
        assert_eq!(result, "# New Heading\n\nSome content");
    }
    
    #[test]
    fn test_list_item_update() {
        let result = MarkdownEditorIntegration::update_markdown_content(
            "- Old item\n- Another item",
            "list_item-0",
            "New item"
        ).unwrap();
        
        assert_eq!(result, "- New item\n- Another item");
    }
}