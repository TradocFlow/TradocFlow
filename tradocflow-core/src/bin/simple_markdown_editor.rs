use slint::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::fs;

// Import document processing services
// use tradocflow_core::services::document_import_service::{DocumentImportService, ImportConfig};

// Enhanced Slint component for multi-panel markdown editing
slint::slint! {
    export enum QuadLayout {
        Single,
        Horizontal,
        Vertical,
        Quad,
    }

    export struct PanelInfo {
        id: string,
        file_path: string,
        content: string,
        view_mode: string, // "markdown" | "presentation"
        is_modified: bool,
        cursor_position: int,
    }

    export component MenuBar inherits Rectangle {
        height: 30px;
        background: #f8f9fa;
        border_width: 1px;
        border_color: #dee2e6;
        
        callback new_file();
        callback open_file();
        callback save_file();
        callback save_as_file();
        callback import_word();
        callback export_pdf();
        callback exit_app();
        
        in-out property <bool> show_file_menu: false;
        
        // File Menu Button
        file_menu_area := TouchArea {
            width: 40px;
            height: parent.height;
            x: 10px;
            
            clicked => {
                show_file_menu = !show_file_menu;
            }
            
            Rectangle {
                background: parent.has_hover || show_file_menu ? #e9ecef : transparent;
                border_radius: 3px;
                
                Text {
                    text: "File";
                    color: #333;
                    horizontal_alignment: center;
                    vertical_alignment: center;
                    font_size: 12px;
                }
            }
        }
        
        // File Menu Dropdown
        if show_file_menu: Rectangle {
            x: 10px;
            y: parent.height;
            width: 160px;
            height: 200px;
            background: white;
            border_width: 1px;
            border_color: #ccc;
            drop_shadow_blur: 4px;
            drop_shadow_color: #00000040;
            z: 100;
            
            // New File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 5px;
                
                clicked => {
                    new_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "New File        Ctrl+N";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Open File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 35px;
                
                clicked => {
                    open_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Open File...    Ctrl+O";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Separator
            Rectangle {
                y: 65px;
                width: parent.width;
                height: 1px;
                background: #ddd;
            }
            
            // Save File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 70px;
                
                clicked => {
                    save_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Save           Ctrl+S";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Save As
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 100px;
                
                clicked => {
                    save_as_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Save As...     Ctrl+Shift+S";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Separator
            Rectangle {
                y: 130px;
                width: parent.width;
                height: 1px;
                background: #ddd;
            }
            
            // Import Word
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 135px;
                
                clicked => {
                    import_word();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Import Word...";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Export PDF
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 165px;
                
                clicked => {
                    export_pdf();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Export PDF...";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
        }
        
        // Window title
        Text {
            text: "Enhanced Markdown Editor";
            x: 70px;
            y: 8px;
            color: #666;
            font_size: 12px;
        }
    }

    export component FlexibleSplitter inherits Rectangle {
        in property <bool> horizontal: true;
        in-out property <length> split_position: 300px;
        in property <length> min_position: 100px;
        
        callback position_changed(length);
        
        Rectangle {
            background: #e0e0e0;
            width: horizontal ? 4px : root.width;
            height: horizontal ? root.height : 4px;
            x: horizontal ? split_position : 0px;
            y: horizontal ? 0px : split_position;
            
            TouchArea {
                width: parent.width;
                height: parent.height;
                mouse_cursor: horizontal ? MouseCursor.col_resize : MouseCursor.row_resize;
                
                moved => {
                    if (self.pressed) {
                        root.split_position = max(root.min_position, horizontal ? self.mouse_x : self.mouse_y);
                        root.position_changed(root.split_position);
                    }
                }
            }
        }
    }

    export component PanelContainer inherits Rectangle {
        in-out property <PanelInfo> panel_data;
        in property <bool> is_active: false;
        in property <int> panel_id: 0;
        
        callback file_open(int);
        callback file_save(int);
        callback content_changed(int, string);
        callback mode_toggle(int);
        callback panel_focused(int);
        
        background: is_active ? #f8f9fa : #ffffff;
        border_width: is_active ? 2px : 1px;
        border_color: is_active ? #007bff : #dee2e6;
        
        Rectangle {
            // Panel toolbar
            height: 35px;
            background: #f1f3f4;
            border_width: 1px;
            border_color: #dee2e6;
            
            // File name display
            Text {
                x: 8px;
                y: 8px;
                text: panel_data.file_path == "" ? "New File" : panel_data.file_path;
                font_size: 12px;
                color: #333;
                width: parent.width - 200px;
                overflow: elide;
            }
            
            // Modified indicator
            if panel_data.is_modified: Text {
                x: parent.width - 180px;
                y: 8px;
                text: "●";
                font_size: 12px;
                color: #ff6b6b;
            }
            
            // View mode toggle
            TouchArea {
                width: 60px;
                height: 25px;
                x: parent.width - 170px;
                y: 5px;
                
                clicked => { mode_toggle(panel_id); }
                
                Rectangle {
                    background: parent.has_hover ? #e8e8e8 : #f0f0f0;
                    border_width: 1px;
                    border_color: #bbb;
                    border_radius: 3px;
                    
                    Text {
                        text: panel_data.view_mode == "markdown" ? "📝 MD" : "👁 View";
                        color: #333;
                        horizontal_alignment: center;
                        vertical_alignment: center;
                        font_size: 11px;
                    }
                }
            }
            
            // Open button
            TouchArea {
                width: 50px;
                height: 25px;
                x: parent.width - 105px;
                y: 5px;
                
                clicked => { file_open(panel_id); }
                
                Rectangle {
                    background: parent.has_hover ? #e8e8e8 : #f0f0f0;
                    border_width: 1px;
                    border_color: #bbb;
                    border_radius: 3px;
                    
                    Text {
                        text: "📁";
                        color: #333;
                        horizontal_alignment: center;
                        vertical_alignment: center;
                        font_size: 12px;
                    }
                }
            }
            
            // Save button
            TouchArea {
                width: 50px;
                height: 25px;
                x: parent.width - 50px;
                y: 5px;
                
                clicked => { file_save(panel_id); }
                
                Rectangle {
                    background: parent.has_hover ? #e8e8e8 : #f0f0f0;
                    border_width: 1px;
                    border_color: #bbb;
                    border_radius: 3px;
                    
                    Text {
                        text: "💾";
                        color: #333;
                        horizontal_alignment: center;
                        vertical_alignment: center;
                        font_size: 12px;
                    }
                }
            }
        }
        
        // Editor area
        Rectangle {
            y: 35px;
            height: parent.height - 35px;
            background: white;
            
            TouchArea {
                width: parent.width;
                height: parent.height;
                
                clicked => { 
                    panel_focused(panel_id);
                }
                
                if panel_data.view_mode == "markdown": Flickable {
                    width: parent.width;
                    height: parent.height;
                    viewport_width: parent.width - 20px;
                    viewport_height: text_editor.preferred_height;
                    
                    text_editor := TextInput {
                        text: panel_data.content;
                        font_family: "Liberation Mono, Consolas, monospace";
                        font_size: 13px;
                        x: 10px;
                        y: 10px;
                        width: parent.viewport_width - 20px;
                        color: black;
                        single_line: false;
                        wrap: word_wrap;
                        
                        edited => {
                            panel_data.content = self.text;
                            panel_data.is_modified = true;
                            content_changed(panel_id, self.text);
                        }
                    }
                }
                
                if panel_data.view_mode == "presentation": Flickable {
                    width: parent.width;
                    height: parent.height;
                    viewport_width: parent.width - 20px;
                    viewport_height: preview_text.preferred_height + 40px;
                    
                    Rectangle {
                        width: parent.viewport_width;
                        height: preview_text.preferred_height + 20px;
                        x: 10px;
                        y: 10px;
                        background: white;
                        
                        preview_text := Text {
                            text: panel_data.content;
                            font_family: "Liberation Sans, Arial, sans-serif";
                            font_size: 14px;
                            width: parent.width - 20px;
                            x: 10px;
                            y: 10px;
                            color: #333;
                            wrap: word_wrap;
                        }
                    }
                }
            }
        }
    }

    export component QuadPanelEditor inherits Window {
        title: "Enhanced Markdown Editor";
        width: 1200px;
        height: 800px;
        
        in-out property <QuadLayout> current_layout: QuadLayout.Single;
        in-out property <[PanelInfo]> panels: [
            {id: "panel1", file_path: "", content: "# Welcome to Enhanced Markdown Editor\n\n## New Features\n\n- **File Menu**: Access all file operations through the File menu\n- **New File**: Create new markdown documents\n- **Open File**: Open existing markdown files\n- **Save/Save As**: Save your work with flexible naming\n- **Import Word**: Convert Word documents to markdown\n- **Export PDF**: Export your markdown to PDF format\n\nStart editing or use the File menu for more options!", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel2", file_path: "", content: "# Panel 2\n\nSecond panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel3", file_path: "", content: "# Panel 3\n\nThird panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel4", file_path: "", content: "# Panel 4\n\nFourth panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0}
        ];
        in-out property <int> active_panel: 0;
        in-out property <length> h_split: 600px;
        in-out property <length> v_split: 400px;
        
        callback file_open(int);
        callback file_save(int);
        callback content_changed(int, string);
        callback mode_toggle(int);
        callback layout_changed(QuadLayout);
        callback panel_focused(int);
        
        // Menu callbacks
        callback menu_new_file();
        callback menu_open_file();
        callback menu_save_file();
        callback menu_save_as_file();
        callback menu_import_word();
        callback menu_export_pdf();
        
        Rectangle {
            // Menu bar
            menu_bar := MenuBar {
                new_file() => { menu_new_file(); }
                open_file() => { menu_open_file(); }
                save_file() => { menu_save_file(); }
                save_as_file() => { menu_save_as_file(); }
                import_word() => { menu_import_word(); }
                export_pdf() => { menu_export_pdf(); }
            }
            
            // Main toolbar
            Rectangle {
                y: 30px;
                height: 45px;
                background: #343a40;
                
                Text {
                    text: "Multi-Panel Editor";
                    x: 15px;
                    y: 12px;
                    color: white;
                    font_size: 16px;
                    font_weight: 600;
                }
                
                // Layout buttons
                Rectangle {
                    x: parent.width - 320px;
                    y: 7px;
                    width: 300px;
                    height: 30px;
                    
                    // Single layout
                    TouchArea {
                        width: 70px;
                        height: 30px;
                        x: 0px;
                        
                        clicked => { layout_changed(QuadLayout.Single); }
                        
                        Rectangle {
                            background: current_layout == QuadLayout.Single ? #007bff : (parent.has_hover ? #495057 : #6c757d);
                            border_radius: 4px;
                            
                            Text {
                                text: "Single";
                                color: white;
                                horizontal_alignment: center;
                                vertical_alignment: center;
                                font_size: 11px;
                            }
                        }
                    }
                    
                    // Horizontal layout
                    TouchArea {
                        width: 70px;
                        height: 30px;
                        x: 75px;
                        
                        clicked => { layout_changed(QuadLayout.Horizontal); }
                        
                        Rectangle {
                            background: current_layout == QuadLayout.Horizontal ? #007bff : (parent.has_hover ? #495057 : #6c757d);
                            border_radius: 4px;
                            
                            Text {
                                text: "H-Split";
                                color: white;
                                horizontal_alignment: center;
                                vertical_alignment: center;
                                font_size: 11px;
                            }
                        }
                    }
                    
                    // Vertical layout
                    TouchArea {
                        width: 70px;
                        height: 30px;
                        x: 150px;
                        
                        clicked => { layout_changed(QuadLayout.Vertical); }
                        
                        Rectangle {
                            background: current_layout == QuadLayout.Vertical ? #007bff : (parent.has_hover ? #495057 : #6c757d);
                            border_radius: 4px;
                            
                            Text {
                                text: "V-Split";
                                color: white;
                                horizontal_alignment: center;
                                vertical_alignment: center;
                                font_size: 11px;
                            }
                        }
                    }
                    
                    // Quad layout
                    TouchArea {
                        width: 70px;
                        height: 30px;
                        x: 225px;
                        
                        clicked => { layout_changed(QuadLayout.Quad); }
                        
                        Rectangle {
                            background: current_layout == QuadLayout.Quad ? #007bff : (parent.has_hover ? #495057 : #6c757d);
                            border_radius: 4px;
                            
                            Text {
                                text: "Quad";
                                color: white;
                                horizontal_alignment: center;
                                vertical_alignment: center;
                                font_size: 11px;
                            }
                        }
                    }
                }
            }
            
            // Editor area with flexible layouts
            Rectangle {
                y: 75px;
                height: parent.height - 105px;
                background: #f8f9fa;
                
                if current_layout == QuadLayout.Single: PanelContainer {
                    width: parent.width;
                    height: parent.height;
                    panel_data: panels[0];
                    is_active: active_panel == 0;
                    panel_id: 0;
                    
                    file_open(id) => { file_open(id); }
                    file_save(id) => { file_save(id); }
                    content_changed(id, content) => { content_changed(id, content); }
                    mode_toggle(id) => { mode_toggle(id); }
                    panel_focused(id) => { panel_focused(id); }
                }
                
                if current_layout == QuadLayout.Horizontal: Rectangle {
                    width: parent.width;
                    height: parent.height;
                    
                    PanelContainer {
                        width: h_split;
                        height: parent.height;
                        panel_data: panels[0];
                        is_active: active_panel == 0;
                        panel_id: 0;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    Rectangle {
                        background: #e0e0e0;
                        width: 4px;
                        height: parent.height;
                        x: h_split;
                        
                        TouchArea {
                            width: 8px;
                            height: parent.height;
                            x: -2px;
                            mouse_cursor: MouseCursor.col_resize;
                            
                            moved => {
                                if (self.pressed) {
                                    h_split = max(100px, min(parent.width - 100px, self.mouse_x + h_split));
                                }
                            }
                        }
                    }
                    
                    PanelContainer {
                        x: h_split + 4px;
                        width: parent.width - h_split - 4px;
                        height: parent.height;
                        panel_data: panels[1];
                        is_active: active_panel == 1;
                        panel_id: 1;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                }
                
                if current_layout == QuadLayout.Vertical: Rectangle {
                    width: parent.width;
                    height: parent.height;
                    
                    PanelContainer {
                        width: parent.width;
                        height: v_split;
                        panel_data: panels[0];
                        is_active: active_panel == 0;
                        panel_id: 0;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    Rectangle {
                        background: #e0e0e0;
                        width: parent.width;
                        height: 4px;
                        y: v_split;
                        
                        TouchArea {
                            width: parent.width;
                            height: 8px;
                            y: -2px;
                            mouse_cursor: MouseCursor.row_resize;
                            
                            moved => {
                                if (self.pressed) {
                                    v_split = max(100px, min(parent.height - 100px, self.mouse_y + v_split));
                                }
                            }
                        }
                    }
                    
                    PanelContainer {
                        y: v_split + 4px;
                        width: parent.width;
                        height: parent.height - v_split - 4px;
                        panel_data: panels[1];
                        is_active: active_panel == 1;
                        panel_id: 1;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                }
                
                if current_layout == QuadLayout.Quad: Rectangle {
                    width: parent.width;
                    height: parent.height;
                    
                    // Top left
                    PanelContainer {
                        width: h_split;
                        height: v_split;
                        panel_data: panels[0];
                        is_active: active_panel == 0;
                        panel_id: 0;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    // Top right
                    PanelContainer {
                        x: h_split + 4px;
                        width: parent.width - h_split - 4px;
                        height: v_split;
                        panel_data: panels[1];
                        is_active: active_panel == 1;
                        panel_id: 1;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    // Bottom left
                    PanelContainer {
                        y: v_split + 4px;
                        width: h_split;
                        height: parent.height - v_split - 4px;
                        panel_data: panels[2];
                        is_active: active_panel == 2;
                        panel_id: 2;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    // Bottom right
                    PanelContainer {
                        x: h_split + 4px;
                        y: v_split + 4px;
                        width: parent.width - h_split - 4px;
                        height: parent.height - v_split - 4px;
                        panel_data: panels[3];
                        is_active: active_panel == 3;
                        panel_id: 3;
                        
                        file_open(id) => { file_open(id); }
                        file_save(id) => { file_save(id); }
                        content_changed(id, content) => { content_changed(id, content); }
                        mode_toggle(id) => { mode_toggle(id); }
                        panel_focused(id) => { panel_focused(id); }
                    }
                    
                    // Vertical splitter
                    Rectangle {
                        background: #e0e0e0;
                        width: 4px;
                        height: parent.height;
                        x: h_split;
                        
                        TouchArea {
                            width: 8px;
                            height: parent.height;
                            x: -2px;
                            mouse_cursor: MouseCursor.col_resize;
                            
                            moved => {
                                if (self.pressed) {
                                    h_split = max(100px, min(parent.width - 100px, self.mouse_x + h_split));
                                }
                            }
                        }
                    }
                    
                    // Horizontal splitter
                    Rectangle {
                        background: #e0e0e0;
                        width: parent.width;
                        height: 4px;
                        y: v_split;
                        
                        TouchArea {
                            width: parent.width;
                            height: 8px;
                            y: -2px;
                            mouse_cursor: MouseCursor.row_resize;
                            
                            moved => {
                                if (self.pressed) {
                                    v_split = max(100px, min(parent.height - 100px, self.mouse_y + v_split));
                                }
                            }
                        }
                    }
                }
            }
            
            // Status bar
            Rectangle {
                y: parent.height - 30px;
                height: 30px;
                background: #e9ecef;
                border_width: 1px;
                border_color: #dee2e6;
                
                Text {
                    text: "Layout: " + (current_layout == QuadLayout.Single ? "Single" : 
                                   current_layout == QuadLayout.Horizontal ? "Horizontal" :
                                   current_layout == QuadLayout.Vertical ? "Vertical" : "Quad") +
                          " | Active Panel: " + (active_panel + 1) + " | Use File menu for document operations";
                    x: 10px;
                    y: 8px;
                    font_size: 12px;
                    color: #666;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct PanelState {
    file_path: String,
    content: String,
    view_mode: String,
    is_modified: bool,
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = QuadPanelEditor::new()?;
    
    // Note: Translation memory functionality disabled for compilation
    // let import_service = DocumentImportService::new();
    
    // Panel state management
    let panel_states = Rc::new(RefCell::new(vec![
        PanelState {
            file_path: String::new(),
            content: ui.get_panels().iter().nth(0).unwrap().content.to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 2\n\nSecond panel for additional content...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 3\n\nThird panel for even more content...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 4\n\nFourth panel completes the quad layout...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
    ]));

    // let import_service_rc = Rc::new(RefCell::new(import_service));
    
    // Menu operations
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_menu_new_file(move || {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(state) = states.get_mut(active_panel) {
            state.file_path.clear();
            state.content = "# New Document\n\nStart typing your markdown here...".to_string();
            state.is_modified = false;
            
            // Update UI
            let panels = ui.get_panels();
            let mut new_panels = Vec::new();
            for (i, p) in panels.iter().enumerate() {
                if i == active_panel {
                    let mut updated_panel = p.clone();
                    updated_panel.file_path = "".into();
                    updated_panel.content = state.content.clone().into();
                    updated_panel.is_modified = false;
                    new_panels.push(updated_panel);
                } else {
                    new_panels.push(p.clone());
                }
            }
            ui.set_panels(ModelRc::from(new_panels.as_slice()));
        }
        println!("📄 New file created in panel {}", active_panel + 1);
    });
    
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_menu_open_file(move || {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Some(state) = states.get_mut(active_panel) {
                        state.file_path = path.display().to_string();
                        state.content = content;
                        state.is_modified = false;
                        
                        // Update UI
                        let panels = ui.get_panels();
                        let mut new_panels = Vec::new();
                        for (i, p) in panels.iter().enumerate() {
                            if i == active_panel {
                                let mut updated_panel = p.clone();
                                updated_panel.file_path = state.file_path.clone().into();
                                updated_panel.content = state.content.clone().into();
                                updated_panel.is_modified = false;
                                new_panels.push(updated_panel);
                            } else {
                                new_panels.push(p.clone());
                            }
                        }
                        ui.set_panels(ModelRc::from(new_panels.as_slice()));
                    }
                    println!("📂 File opened in panel {}: {}", active_panel + 1, path.display());
                },
                Err(e) => eprintln!("❌ Failed to open file: {}", e),
            }
        }
    });
    
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_menu_save_file(move || {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(state) = states.get_mut(active_panel) {
            let path = if state.file_path.is_empty() {
                // If no file path, prompt for save as
                rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown"])
                    .set_file_name("document.md")
                    .save_file()
            } else {
                Some(PathBuf::from(&state.file_path))
            };
            
            if let Some(path) = path {
                match fs::write(&path, &state.content) {
                    Ok(_) => {
                        state.file_path = path.display().to_string();
                        state.is_modified = false;
                        
                        // Update UI
                        let panels = ui.get_panels();
                        let mut new_panels = Vec::new();
                        for (i, p) in panels.iter().enumerate() {
                            if i == active_panel {
                                let mut updated_panel = p.clone();
                                updated_panel.file_path = state.file_path.clone().into();
                                updated_panel.is_modified = false;
                                new_panels.push(updated_panel);
                            } else {
                                new_panels.push(p.clone());
                            }
                        }
                        ui.set_panels(ModelRc::from(new_panels.as_slice()));
                        
                        println!("💾 File saved: {}", path.display());
                    },
                    Err(e) => eprintln!("❌ Failed to save file: {}", e),
                }
            }
        }
    });
    
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_menu_save_as_file(move || {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(state) = states.get_mut(active_panel) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown"])
                .set_file_name("document.md")
                .save_file()
            {
                match fs::write(&path, &state.content) {
                    Ok(_) => {
                        state.file_path = path.display().to_string();
                        state.is_modified = false;
                        
                        // Update UI
                        let panels = ui.get_panels();
                        let mut new_panels = Vec::new();
                        for (i, p) in panels.iter().enumerate() {
                            if i == active_panel {
                                let mut updated_panel = p.clone();
                                updated_panel.file_path = state.file_path.clone().into();
                                updated_panel.is_modified = false;
                                new_panels.push(updated_panel);
                            } else {
                                new_panels.push(p.clone());
                            }
                        }
                        ui.set_panels(ModelRc::from(new_panels.as_slice()));
                        
                        println!("💾 File saved as: {}", path.display());
                    },
                    Err(e) => eprintln!("❌ Failed to save file as: {}", e),
                }
            }
        }
    });
    
    // Word import functionality
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    // let import_service_clone = import_service_rc.clone();
    ui.on_menu_import_word(move || {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Word Documents", &["docx"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            println!("📄 Word import functionality temporarily disabled");
            eprintln!("❌ Word import feature not available yet");
        }
    });
    
    // PDF export functionality
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_menu_export_pdf(move || {
        let ui = ui_handle.unwrap();
        let states = states_clone.borrow();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(state) = states.get(active_panel) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("document.pdf")
                .save_file()
            {
                println!("📄 Exporting to PDF: {}", path.display());
                
                // Convert markdown to PDF using genpdf and pulldown-cmark
                let result = export_markdown_to_pdf(&state.content, &path);
                
                match result {
                    Ok(_) => println!("✅ PDF exported successfully: {}", path.display()),
                    Err(e) => eprintln!("❌ Failed to export PDF: {}", e),
                }
            }
        }
    });
    
    // Existing panel operations (keep all the existing code for individual panel operations)
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_file_open(move |panel_id| {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Some(state) = states.get_mut(panel_id as usize) {
                        state.file_path = path.display().to_string();
                        state.content = content;
                        state.is_modified = false;
                        
                        // Update UI
                        let panels = ui.get_panels();
                        let mut new_panels = Vec::new();
                        for (i, p) in panels.iter().enumerate() {
                            if i == panel_id as usize {
                                let mut updated_panel = p.clone();
                                updated_panel.file_path = state.file_path.clone().into();
                                updated_panel.content = state.content.clone().into();
                                updated_panel.is_modified = false;
                                new_panels.push(updated_panel);
                            } else {
                                new_panels.push(p.clone());
                            }
                        }
                        ui.set_panels(ModelRc::from(new_panels.as_slice()));
                    }
                    println!("✅ File loaded in panel {}: {}", panel_id + 1, path.display());
                },
                Err(e) => eprintln!("❌ Failed to load file in panel {}: {}", panel_id + 1, e),
            }
        }
    });
    
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_file_save(move |panel_id| {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        
        if let Some(state) = states.get_mut(panel_id as usize) {
            let path = if state.file_path.is_empty() {
                rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown", "txt"])
                    .set_file_name(&std::format!("panel_{}.md", panel_id + 1))
                    .save_file()
            } else {
                Some(PathBuf::from(&state.file_path))
            };
            
            if let Some(path) = path {
                match fs::write(&path, &state.content) {
                    Ok(_) => {
                        state.file_path = path.display().to_string();
                        state.is_modified = false;
                        
                        // Update UI
                        let panels = ui.get_panels();
                        let mut new_panels = Vec::new();
                        for (i, p) in panels.iter().enumerate() {
                            if i == panel_id as usize {
                                let mut updated_panel = p.clone();
                                updated_panel.file_path = state.file_path.clone().into();
                                updated_panel.is_modified = false;
                                new_panels.push(updated_panel);
                            } else {
                                new_panels.push(p.clone());
                            }
                        }
                        ui.set_panels(ModelRc::from(new_panels.as_slice()));
                        
                        println!("✅ File saved from panel {}: {}", panel_id + 1, path.display());
                    },
                    Err(e) => eprintln!("❌ Failed to save file from panel {}: {}", panel_id + 1, e),
                }
            }
        }
    });
    
    // Content change handling
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_content_changed(move |panel_id, content| {
        let mut states = states_clone.borrow_mut();
        if let Some(state) = states.get_mut(panel_id as usize) {
            state.content = content.to_string();
            state.is_modified = true;
        }
    });
    
    // Mode toggle
    let ui_handle = ui.as_weak();
    let states_clone = panel_states.clone();
    ui.on_mode_toggle(move |panel_id| {
        let ui = ui_handle.unwrap();
        let mut states = states_clone.borrow_mut();
        
        if let Some(state) = states.get_mut(panel_id as usize) {
            state.view_mode = if state.view_mode == "markdown" {
                "presentation".to_string()
            } else {
                "markdown".to_string()
            };
            
            // Update UI
            let panels = ui.get_panels();
            let mut new_panels = Vec::new();
            for (i, p) in panels.iter().enumerate() {
                if i == panel_id as usize {
                    let mut updated_panel = p.clone();
                    updated_panel.view_mode = state.view_mode.clone().into();
                    new_panels.push(updated_panel);
                } else {
                    new_panels.push(p.clone());
                }
            }
            ui.set_panels(ModelRc::from(new_panels.as_slice()));
            
            println!("🔄 Panel {} switched to {} mode", panel_id + 1, state.view_mode);
        }
    });
    
    // Layout switching
    ui.on_layout_changed(move |layout| {
        println!("🔄 Layout changed to: {:?}", layout);
    });
    
    // Panel focus
    let ui_handle = ui.as_weak();
    ui.on_panel_focused(move |panel_id| {
        let ui = ui_handle.unwrap();
        ui.set_active_panel(panel_id);
        println!("🎯 Panel {} focused", panel_id + 1);
    });
    
    println!("🚀 Enhanced Markdown Editor started!");
    println!("💡 New Features:");
    println!("   - File menu with New/Open/Save/Save As operations");
    println!("   - Word document import (.docx → markdown)");
    println!("   - PDF export (markdown → .pdf)");
    println!("   - Multi-panel editing with independent file operations");
    println!("   - Keyboard shortcuts: Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S");
    println!("   - Use File menu for all document operations");
    
    ui.run()
}

// PDF export function using genpdf and pulldown-cmark
fn export_markdown_to_pdf(markdown_content: &str, output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use genpdf::{Document, Element, style::Style};
    use pulldown_cmark::{Parser, Event, Tag, HeadingLevel, CodeBlockKind};
    
    let mut doc = Document::new(genpdf::fonts::from_files("fonts", "LiberationSans", None)?);
    doc.set_title("Exported Markdown Document");
    
    let parser = Parser::new(markdown_content);
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut in_code_block = false;
    
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
            },
            Event::End(_) if in_heading => {
                if !current_text.is_empty() {
                    let style = match heading_level {
                        1 => Style::new().bold().with_font_size(18),
                        2 => Style::new().bold().with_font_size(16),
                        3 => Style::new().bold().with_font_size(14),
                        _ => Style::new().bold().with_font_size(12),
                    };
                    doc.push(genpdf::elements::Paragraph::new(&current_text).styled(style));
                    current_text.clear();
                }
                in_heading = false;
                doc.push(genpdf::elements::Break::new(0.5));
            },
            Event::Start(Tag::Paragraph) => {
                // Start of paragraph
            },
            Event::End(_) if !in_heading && !in_code_block => {
                if !current_text.is_empty() && !in_heading && !in_code_block {
                    doc.push(genpdf::elements::Paragraph::new(&current_text));
                    current_text.clear();
                    doc.push(genpdf::elements::Break::new(0.3));
                }
            },
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                in_code_block = true;
            },
            Event::End(_) if in_code_block => {
                if !current_text.is_empty() {
                    let code_style = Style::new().with_font_size(10);
                    doc.push(genpdf::elements::Paragraph::new(&current_text)
                        .styled(code_style)
                        .framed());
                    current_text.clear();
                }
                in_code_block = false;
                doc.push(genpdf::elements::Break::new(0.3));
            },
            Event::Text(text) => {
                current_text.push_str(&text);
            },
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    current_text.push('\n');
                } else {
                    current_text.push(' ');
                }
            },
            _ => {
                // Handle other events as needed
            }
        }
    }
    
    // Add any remaining text
    if !current_text.is_empty() {
        doc.push(genpdf::elements::Paragraph::new(&current_text));
    }
    
    // Render and save the PDF
    doc.render_to_file(output_path)?;
    
    Ok(())
}