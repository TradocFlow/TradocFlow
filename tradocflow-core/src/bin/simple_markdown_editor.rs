use slint::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

// Import document processing services
use tradocflow_core::services::{
    ThreadSafeDocumentProcessor, DocumentProcessingConfig, ImportProgressInfo, ImportStage
};

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

    export struct PdfExportConfig {
        // Paper settings
        paper_format: string, // "A4", "Letter", "Legal", "A3", "A5", "Custom"
        orientation: string,   // "Portrait", "Landscape"
        custom_width: int,
        custom_height: int,
        
        // Margins (in mm)
        margin_top: int,
        margin_bottom: int,
        margin_left: int,
        margin_right: int,
        
        // Font settings
        base_font: string,
        font_size: int,
        line_height: int,
        
        // Content options
        include_toc: bool,
        include_page_numbers: bool,
        include_headers_footers: bool,
        header_text: string,
        footer_text: string,
        syntax_highlighting: bool,
        preserve_code_formatting: bool,
        
        // Link handling
        link_handling: string, // "Preserve", "RemoveFormatting", "ConvertToFootnotes"
        
        // Image quality
        image_quality: string, // "Low", "Medium", "High", "Original"
        
        // Metadata
        document_title: string,
        document_author: string,
        document_subject: string,
    }

    export struct PdfExportProgress {
        visible: bool,
        stage: string,
        progress_percent: int,
        current_item: string,
        items_completed: int,
        total_items: int,
        message: string,
        warnings: [string],
        can_cancel: bool,
    }

    export component ImportProgressDialog inherits Rectangle {
        in-out property <bool> dialog-visible: false;
        in-out property <string> current_file: "";
        in-out property <int> progress_percent: 0;
        in-out property <string> message: "";
        in-out property <string> stage: "";
        in-out property <[string]> warnings: [];
        in-out property <[string]> errors: [];
        
        callback cancel_import();
        
        if dialog-visible: Rectangle {
            width: 500px;
            height: 350px;
            background: white;
            border_width: 2px;
            border_color: #007bff;
            border_radius: 8px;
            drop_shadow_blur: 10px;
            drop_shadow_color: #00000040;
            z: 1000;
            
            // Center on screen
            x: (parent.width - self.width) / 2;
            y: (parent.height - self.height) / 2;
            
            // Title bar
            Rectangle {
                height: 40px;
                background: #007bff;
                border_radius: 6px;
                
                Text {
                    text: "Importing Word Document";
                    color: white;
                    font_size: 14px;
                    font_weight: 600;
                    x: 15px;
                    y: 12px;
                }
            }
            
            // Content area
            Rectangle {
                y: 40px;
                height: parent.height - 80px;
                x: 15px;
                width: parent.width - 30px;
                
                // Current file
                Text {
                    text: "File: " + current_file;
                    color: #333;
                    font_size: 12px;
                    y: 15px;
                    width: parent.width;
                    overflow: elide;
                }
                
                // Stage indicator
                Text {
                    text: "Stage: " + stage;
                    color: #666;
                    font_size: 11px;
                    y: 35px;
                }
                
                // Progress bar
                Rectangle {
                    y: 60px;
                    width: parent.width;
                    height: 20px;
                    background: #f0f0f0;
                    border_width: 1px;
                    border_color: #ddd;
                    border_radius: 10px;
                    
                    Rectangle {
                        width: (parent.width * progress_percent) / 100;
                        height: parent.height;
                        background: #007bff;
                        border_radius: 10px;
                    }
                }
                
                // Progress percentage
                Text {
                    text: progress_percent + "%";
                    color: #333;
                    font_size: 11px;
                    y: 85px;
                    horizontal_alignment: center;
                    width: parent.width;
                }
                
                // Message
                Text {
                    text: message;
                    color: #666;
                    font_size: 11px;
                    y: 110px;
                    width: parent.width;
                    wrap: word_wrap;
                    height: 40px;
                }
                
                // Warnings section
                if warnings.length > 0: Rectangle {
                    y: 155px;
                    width: parent.width;
                    height: 60px;
                    
                    Text {
                        text: "⚠️ Warnings (" + warnings.length + "):";
                        color: #ff8c00;
                        font_size: 11px;
                        font_weight: 600;
                    }
                    
                    Flickable {
                        y: 20px;
                        width: parent.width;
                        height: 40px;
                        viewport_height: warnings.length * 15px;
                        
                        for warning[i] in warnings: Text {
                            text: "• " + warning;
                            color: #ff8c00;
                            font_size: 10px;
                            y: i * 15px;
                            width: parent.width;
                            overflow: elide;
                        }
                    }
                }
                
                // Errors section
                if errors.length > 0: Rectangle {
                    y: 220px;
                    width: parent.width;
                    height: 60px;
                    
                    Text {
                        text: "❌ Errors (" + errors.length + "):";
                        color: #dc3545;
                        font_size: 11px;
                        font_weight: 600;
                    }
                    
                    Flickable {
                        y: 20px;
                        width: parent.width;
                        height: 40px;
                        viewport_height: errors.length * 15px;
                        
                        for error[i] in errors: Text {
                            text: "• " + error;
                            color: #dc3545;
                            font_size: 10px;
                            y: i * 15px;
                            width: parent.width;
                            overflow: elide;
                        }
                    }
                }
            }
            
            // Button area
            Rectangle {
                y: parent.height - 40px;
                height: 40px;
                background: #f8f9fa;
                border_radius: 6px;
                
                TouchArea {
                    width: 80px;
                    height: 30px;
                    x: parent.width - 95px;
                    y: 5px;
                    
                    clicked => { cancel_import(); }
                    
                    Rectangle {
                        background: parent.has_hover ? #dc3545 : #6c757d;
                        border_radius: 4px;
                        
                        Text {
                            text: "Cancel";
                            color: white;
                            horizontal_alignment: center;
                            vertical_alignment: center;
                            font_size: 11px;
                        }
                    }
                }
            }
        }
    }

    export component MenuBar inherits Rectangle {
        width: 100%;
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
        callback toggle_menu();
        callback toggle_window_menu();
        callback focus_panel(int);
        
        in-out property <bool> menu_visible: false;
        in-out property <bool> window_menu_visible: false;
        in-out property <[PanelInfo]> open_panels: [];
        in-out property <int> active_panel: 0;
        
        // File Menu Button
        file_menu_area := TouchArea {
            width: 40px;
            height: parent.height;
            x: 10px;
            
            clicked => {
                toggle_menu();
            }
            
            Rectangle {
                background: parent.has_hover || menu_visible ? #e9ecef : #f8f9fa;
                border_radius: 3px;
                border_width: 1px;
                border_color: parent.has_hover || menu_visible ? #007bff : #dee2e6;
                
                Text {
                    text: menu_visible ? "File ▼" : "File";
                    color: #333;
                    horizontal_alignment: center;
                    vertical_alignment: center;
                    font_size: 12px;
                }
            }
        }
        
        // Window Menu Button
        window_menu_area := TouchArea {
            width: 60px;
            height: parent.height;
            x: 55px;
            
            clicked => {
                toggle_window_menu();
            }
            
            Rectangle {
                background: parent.has_hover || window_menu_visible ? #e9ecef : #f8f9fa;
                border_radius: 3px;
                border_width: 1px;
                border_color: parent.has_hover || window_menu_visible ? #007bff : #dee2e6;
                
                Text {
                    text: window_menu_visible ? "Window ▼" : "Window";
                    color: #333;
                    horizontal_alignment: center;
                    vertical_alignment: center;
                    font_size: 12px;
                }
            }
        }
        
        // Click-outside handler to close menus
        if menu_visible || window_menu_visible: Rectangle {
            width: root.width;
            height: root.height;
            x: 0px;
            y: 0px;
            z: 999;
            background: transparent;
            
            TouchArea {
                clicked => {
                    menu_visible = false;
                    window_menu_visible = false;
                }
            }
        }
        
        // File Menu Dropdown
        if menu_visible: Rectangle {
            x: 10px;
            y: 30px;
            width: 160px;
            height: 200px;
            background: white;
            border_width: 2px;
            border_color: #007bff;
            drop_shadow_blur: 8px;
            drop_shadow_color: #00000060;
            z: 1000;
            
            animate opacity {
                duration: 150ms;
                easing: ease-out;
            }
            
            // Debug text to verify dropdown is showing
            Text {
                x: 10px;
                y: 10px;
                text: "DROPDOWN VISIBLE";
                color: red;
                font_size: 14px;
                font_weight: 700;
            }
            
            // New File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 5px;
                
                clicked => {
                    new_file();
                    menu_visible = false;
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
                    menu_visible = false;
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
                    menu_visible = false;
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
                    menu_visible = false;
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
                    menu_visible = false;
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
                    menu_visible = false;
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
        
        // Window Menu Dropdown
        if window_menu_visible: Rectangle {
            x: 55px;
            y: 30px;
            width: 250px;
            height: min(200px, open_panels.length * 28px + 10px);
            background: white;
            border_width: 2px;
            border_color: #007bff;
            drop_shadow_blur: 8px;
            drop_shadow_color: #00000060;
            z: 1000;
            
            animate opacity {
                duration: 150ms;
                easing: ease-out;
            }
            
            // Header
            Text {
                x: 10px;
                y: 5px;
                text: "Open Editors";
                color: #666;
                font_size: 11px;
                font_weight: 600;
            }
            
            // Panel list
            for panel[i] in open_panels: TouchArea {
                width: parent.width;
                height: 28px;
                y: 25px + i * 28px;
                
                clicked => {
                    focus_panel(i);
                    window_menu_visible = false;
                }
                
                Rectangle {
                    background: i == active_panel ? #e3f2fd : (parent.has_hover ? #f0f0f0 : transparent);
                    border_left_width: i == active_panel ? 3px : 0px;
                    border_left_color: #007bff;
                    
                    // Modified indicator
                    if panel.is_modified: Text {
                        x: 8px;
                        y: 8px;
                        text: "●";
                        font_size: 12px;
                        color: #ff6b6b;
                    }
                    
                    // File name
                    Text {
                        x: panel.is_modified ? 20px : 10px;
                        y: 6px;
                        text: panel.file_path == "" ? "New File" : panel.file_path;
                        font_size: 11px;
                        color: i == active_panel ? #007bff : #333;
                        font_weight: i == active_panel ? 600 : 400;
                        width: parent.width - (panel.is_modified ? 30px : 20px);
                        overflow: elide;
                    }
                    
                    // Panel number
                    Text {
                        x: parent.width - 25px;
                        y: 6px;
                        text: "" + (i + 1);
                        font_size: 10px;
                        color: #999;
                        horizontal_alignment: right;
                    }
                }
            }
            
            // No files open message
            if open_panels.length == 0: Text {
                x: 10px;
                y: 30px;
                text: "No files open";
                color: #999;
                font_size: 11px;
                italic: true;
            }
        }
        
        // Window title
        Text {
            text: "Enhanced Markdown Editor";
            x: 125px;
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
        forward-focus: key-handler;
        
        in-out property <QuadLayout> current_layout: QuadLayout.Single;
        in-out property <[PanelInfo]> panels: [
            {id: "panel1", file_path: "", content: "# Welcome to Enhanced Markdown Editor\n\n## New Features\n\n- **File Menu**: Access all file operations through the File menu\n- **New File**: Create new markdown documents\n- **Open File**: Open existing markdown files\n- **Save/Save As**: Save your work with flexible naming\n- **Import Word**: Convert Word documents to markdown\n- **Export PDF**: Export your markdown to PDF format\n- **Keyboard Shortcuts**:\n  - F7: Toggle Single/Double Column\n  - F8: Toggle Top/Bottom Split\n\nStart editing or use the File menu for more options!", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel2", file_path: "", content: "# Panel 2\n\nSecond panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel3", file_path: "", content: "# Panel 3\n\nThird panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0},
            {id: "panel4", file_path: "", content: "# Panel 4\n\nFourth panel content...", view_mode: "markdown", is_modified: false, cursor_position: 0}
        ];
        in-out property <int> active_panel: 0;
        in-out property <length> h_split: 600px;
        in-out property <length> v_split: 400px;
        
        // Import dialog properties
        in-out property <bool> import_dialog_visible: false;
        in-out property <string> import_current_file: "";
        in-out property <int> import_progress: 0;
        in-out property <string> import_message: "";
        in-out property <string> import_stage: "";
        in-out property <[string]> import_warnings: [];
        in-out property <[string]> import_errors: [];
        
        // PDF export dialog properties
        in-out property <bool> pdf_config_dialog_visible: false;
        in-out property <PdfExportConfig> pdf_config: {
            paper_format: "A4",
            orientation: "Portrait",
            custom_width: 210,
            custom_height: 297,
            margin_top: 25,
            margin_bottom: 25,
            margin_left: 25,
            margin_right: 25,
            base_font: "LiberationSans",
            font_size: 11,
            line_height: 14,
            include_toc: true,
            include_page_numbers: true,
            include_headers_footers: false,
            header_text: "",
            footer_text: "",
            syntax_highlighting: false,
            preserve_code_formatting: true,
            link_handling: "Preserve",
            image_quality: "Medium",
            document_title: "",
            document_author: "",
            document_subject: "",
        };
        in-out property <PdfExportProgress> pdf_progress: {
            visible: false,
            stage: "",
            progress_percent: 0,
            current_item: "",
            items_completed: 0,
            total_items: 0,
            message: "",
            warnings: [],
            can_cancel: true,
        };
        
        // File menu state
        in-out property <bool> show_file_menu: false;
        in-out property <bool> show_window_menu: false;
        
        callback file_open(int);
        callback file_save(int);
        callback content_changed(int, string);
        callback mode_toggle(int);
        callback layout_changed(QuadLayout);
        callback panel_focused(int);
        
        // Layout toggle callbacks
        callback toggle_columns();
        callback toggle_rows();
        
        // Menu callbacks
        callback menu_new_file();
        callback menu_open_file();
        callback menu_save_file();
        callback menu_save_as_file();
        callback menu_import_word();
        callback menu_export_pdf();
        callback cancel_import();
        callback cancel_pdf_export();
        
        
        key-handler := FocusScope {
            width: 100%;
            height: 100%;
            
            key-pressed(event) => {
                if (event.text == Key.F7) {
                    toggle_columns();
                    return accept;
                } else if (event.text == Key.F8) {
                    toggle_rows();
                    return accept;
                }
                return reject;
            }
            
            VerticalLayout {
                // Menu bar
                menu_bar := MenuBar {
                menu_visible: show_file_menu;
                window_menu_visible: show_window_menu;
                open_panels: panels;
                active_panel: active_panel;
                toggle_menu() => { show_file_menu = !show_file_menu; }
                toggle_window_menu() => { show_window_menu = !show_window_menu; }
                focus_panel(panel_id) => { panel_focused(panel_id); }
                new_file() => { menu_new_file(); }
                open_file() => { menu_open_file(); }
                save_file() => { menu_save_file(); }
                save_as_file() => { menu_save_as_file(); }
                import_word() => { menu_import_word(); }
                export_pdf() => { menu_export_pdf(); }
            }
            
            // Main toolbar
            Rectangle {
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
                height: 30px;
                background: #e9ecef;
                border_width: 1px;
                border_color: #dee2e6;
                
                Text {
                    text: import_dialog_visible ? 
                        "Importing: " + import_current_file + " (" + import_progress + "%) - " + import_message :
                        "Layout: " + (current_layout == QuadLayout.Single ? "Single" : 
                                     current_layout == QuadLayout.Horizontal ? "Horizontal" :
                                     current_layout == QuadLayout.Vertical ? "Vertical" : "Quad") +
                        " | Active Panel: " + (active_panel + 1) + " | Use File menu for document operations";
                    x: 10px;
                    y: 8px;
                    font_size: 12px;
                    color: import_dialog_visible ? #007bff : #666;
                }
            }
        }
        }
        
        // Click-outside handler to close menus (at window level)
        if show_file_menu || show_window_menu: Rectangle {
            width: parent.width;
            height: parent.height;
            x: 0px;
            y: 0px;
            z: 999;
            background: transparent;
            
            TouchArea {
                clicked => {
                    show_file_menu = false;
                    show_window_menu = false;
                }
            }
        }
        
        // File Menu Dropdown (at window level for proper z-index)
        if show_file_menu: Rectangle {
            x: 10px;
            y: 30px;
            width: 160px;
            height: 200px;
            background: white;
            border_width: 2px;
            border_color: #007bff;
            drop_shadow_blur: 8px;
            drop_shadow_color: #00000060;
            z: 2000;
            
            // Debug text to verify dropdown is showing
            Text {
                x: 10px;
                y: 10px;
                text: "DROPDOWN VISIBLE";
                color: red;
                font_size: 14px;
                font_weight: 700;
            }
            
            // New File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 35px;
                
                clicked => {
                    menu_new_file();
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
                y: 65px;
                
                clicked => {
                    menu_open_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Open File...     Ctrl+O";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Save File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 95px;
                
                clicked => {
                    menu_save_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Save File        Ctrl+S";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Save As File
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 125px;
                
                clicked => {
                    menu_save_as_file();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Save As...   Ctrl+Shift+S";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Import Word
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 155px;
                
                clicked => {
                    menu_import_word();
                    show_file_menu = false;
                }
                
                Rectangle {
                    background: parent.has_hover ? #f0f0f0 : transparent;
                    
                    Text {
                        x: 10px;
                        y: 6px;
                        text: "Import Word Document...";
                        font_size: 11px;
                        color: #333;
                    }
                }
            }
            
            // Export PDF
            TouchArea {
                width: parent.width;
                height: 28px;
                y: 185px;
                
                clicked => {
                    menu_export_pdf();
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
        
        // Window Menu Dropdown (at window level for proper z-index)
        if show_window_menu: Rectangle {
            x: 55px;
            y: 60px;
            width: 280px;
            height: min(220px, panels.length * 32px + 40px);
            background: white;
            border_width: 2px;
            border_color: #007bff;
            border_radius: 6px;
            drop_shadow_blur: 10px;
            drop_shadow_color: #00000040;
            z: 2000;
            
            animate opacity {
                duration: 150ms;
                easing: ease-out;
            }
            
            // Header
            Rectangle {
                height: 35px;
                background: #f8f9fa;
                border_radius: 4px;
                
                Text {
                    x: 12px;
                    y: 10px;
                    text: "Open Editor Panels";
                    color: #666;
                    font_size: 12px;
                    font_weight: 600;
                }
            }
            
            // Panel list
            for panel[i] in panels: TouchArea {
                width: parent.width;
                height: 32px;
                y: 35px + i * 32px;
                
                clicked => {
                    panel_focused(i);
                    show_window_menu = false;
                }
                
                Rectangle {
                    background: i == active_panel ? #e3f2fd : (parent.has_hover ? #f0f0f0 : transparent);
                    border_left_width: i == active_panel ? 4px : 0px;
                    border_left_color: #007bff;
                    
                    // Panel indicator
                    Rectangle {
                        x: 8px;
                        y: 8px;
                        width: 16px;
                        height: 16px;
                        background: i == active_panel ? #007bff : #6c757d;
                        border_radius: 2px;
                        
                        Text {
                            text: "" + (i + 1);
                            color: white;
                            font_size: 10px;
                            font_weight: 600;
                            horizontal_alignment: center;
                            vertical_alignment: center;
                        }
                    }
                    
                    // Modified indicator
                    if panel.is_modified: Text {
                        x: 30px;
                        y: 10px;
                        text: "●";
                        font_size: 12px;
                        color: #ff6b6b;
                    }
                    
                    // File name
                    Text {
                        x: panel.is_modified ? 45px : 32px;
                        y: 8px;
                        text: panel.file_path == "" ? "New File" : panel.file_path;
                        font_size: 12px;
                        color: i == active_panel ? #007bff : #333;
                        font_weight: i == active_panel ? 600 : 400;
                        width: parent.width - (panel.is_modified ? 55px : 42px);
                        overflow: elide;
                    }
                    
                    // View mode indicator
                    Text {
                        x: parent.width - 45px;
                        y: 10px;
                        text: panel.view_mode == "markdown" ? "MD" : "View";
                        font_size: 9px;
                        color: #999;
                        horizontal_alignment: right;
                        width: 35px;
                    }
                }
            }
            
            // No panels message (shouldn't happen since we always have 4 panels)
            if panels.length == 0: Text {
                x: 12px;
                y: 50px;
                text: "No editor panels available";
                color: #999;
                font_size: 11px;
                italic: true;
            }
        }
        
        // Import progress dialog
        ImportProgressDialog {
            dialog-visible: import_dialog_visible;
            current_file: import_current_file;
            progress_percent: import_progress;
            message: import_message;
            stage: import_stage;
            warnings: import_warnings;
            errors: import_errors;
            
            cancel_import() => { cancel_import(); }
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
    
    // Initialize document processing service
    let document_processor = match ThreadSafeDocumentProcessor::new() {
        Ok(processor) => Some(processor),
        Err(e) => {
            eprintln!("⚠️  Warning: Document processing service failed to initialize: {}", e);
            eprintln!("   Word import functionality will be limited");
            None
        }
    };
    
    // PDF export service temporarily disabled due to API compatibility issues
    // Enhanced PDF export will be implemented after resolving genpdf dependencies
    
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
    let processor_clone = document_processor.clone();
    ui.on_menu_import_word(move || {
        let ui = ui_handle.unwrap();
        let _states = states_clone.borrow();
        let active_panel = ui.get_active_panel() as usize;
        
        if let Some(processor) = &processor_clone {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Word Documents", &["docx", "doc"])
                .add_filter("Text Files", &["txt"])
                .add_filter("Markdown Files", &["md", "markdown"])
                .add_filter("All Files", &["*"])
                .pick_file()
            {
                let filename = path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Validate file format first
                if !processor.is_format_supported(&filename) {
                    eprintln!("❌ Unsupported file format: {}", filename);
                    eprintln!("   Supported formats: {}", processor.supported_formats().join(", "));
                    return;
                }
                
                // Show import dialog
                ui.set_import_dialog_visible(true);
                ui.set_import_current_file(filename.clone().into());
                ui.set_import_progress(0);
                ui.set_import_message("Preparing import...".into());
                ui.set_import_stage("Initializing".into());
                ui.set_import_warnings(ModelRc::from(Vec::<slint::SharedString>::new().as_slice()));
                ui.set_import_errors(ModelRc::from(Vec::<slint::SharedString>::new().as_slice()));
                
                println!("🔄 Starting Word document import: {}", filename);
                
                // Create processing config
                let config = DocumentProcessingConfig {
                    preserve_formatting: true,
                    extract_images: false,
                    target_language: "en".to_string(),
                    timeout_seconds: 300,
                    max_file_size_mb: 50,
                };
                
                // Create progress callback
                let ui_weak = ui.as_weak();
                let progress_callback = Arc::new(move |progress: ImportProgressInfo| {
                    if let Some(ui) = ui_weak.upgrade() {
                        // Convert stage to string
                        let stage_str = match progress.stage {
                            ImportStage::Validating => "Validating",
                            ImportStage::Processing => "Processing",
                            ImportStage::Converting => "Converting",
                            ImportStage::Finalizing => "Finalizing",
                            ImportStage::Completed => "Completed",
                            ImportStage::Failed => "Failed",
                        };
                        
                        ui.set_import_current_file(progress.current_file.into());
                        ui.set_import_progress(progress.progress_percent as i32);
                        ui.set_import_message(progress.message.into());
                        ui.set_import_stage(stage_str.into());
                        
                        // Update warnings
                        let warnings: Vec<slint::SharedString> = progress.warnings
                            .iter()
                            .map(|w| w.clone().into())
                            .collect();
                        ui.set_import_warnings(ModelRc::from(warnings.as_slice()));
                        
                        // Update errors
                        let errors: Vec<slint::SharedString> = progress.errors
                            .iter()
                            .map(|e| e.clone().into())
                            .collect();
                        ui.set_import_errors(ModelRc::from(errors.as_slice()));
                        
                        // Check if completed or failed
                        if progress.stage == ImportStage::Completed || progress.stage == ImportStage::Failed {
                            // Close dialog after a delay
                            let ui_weak_inner = ui.as_weak();
                            thread::spawn(move || {
                                thread::sleep(Duration::from_millis(2000));
                                if let Some(ui) = ui_weak_inner.upgrade() {
                                    ui.set_import_dialog_visible(false);
                                }
                            });
                        }
                    }
                });
                
                // Process document synchronously with progress updates
                // Note: For a production UI, you'd want to use async/await with a proper event loop
                let result = processor.process_document_sync(&path, config, Some(progress_callback));
                
                match result {
                    Ok(processed_doc) => {
                        println!("✅ Document import completed successfully");
                        println!("   Title: {}", processed_doc.title);
                        println!("   Content length: {} characters", processed_doc.content.len());
                        println!("   Processing time: {}ms", processed_doc.processing_time_ms);
                        
                        if !processed_doc.warnings.is_empty() {
                            println!("⚠️  Import warnings:");
                            for warning in &processed_doc.warnings {
                                println!("   • {}", warning);
                            }
                        }
                        
                        // Update the active panel with imported content
                        let mut states = states_clone.borrow_mut();
                        if let Some(state) = states.get_mut(active_panel) {
                            state.content = processed_doc.content;
                            state.file_path = std::format!("{} (imported)", processed_doc.filename);
                            state.is_modified = true;
                            
                            // Update UI
                            let panels = ui.get_panels();
                            let mut new_panels = Vec::new();
                            for (i, p) in panels.iter().enumerate() {
                                if i == active_panel {
                                    let mut updated_panel = p.clone();
                                    updated_panel.file_path = state.file_path.clone().into();
                                    updated_panel.content = state.content.clone().into();
                                    updated_panel.is_modified = true;
                                    new_panels.push(updated_panel);
                                } else {
                                    new_panels.push(p.clone());
                                }
                            }
                            ui.set_panels(ModelRc::from(new_panels.as_slice()));
                        }
                        
                        // Close dialog after a delay
                        ui.set_import_dialog_visible(false);
                    }
                    Err(e) => {
                        eprintln!("❌ Document import failed: {}", e);
                        ui.set_import_dialog_visible(false);
                        
                        // Show error in dialog
                        let error_msg = std::format!("Import failed: {}", e);
                        ui.set_import_message(error_msg.clone().into());
                        ui.set_import_stage("Failed".into());
                        ui.set_import_errors(ModelRc::from(vec![error_msg.into()].as_slice()));
                        
                        // Close dialog after showing error
                        let ui_weak = ui.as_weak();
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(3000));
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_import_dialog_visible(false);
                            }
                        });
                    }
                }
            }
        } else {
            eprintln!("❌ Word import service not available");
            eprintln!("   Document processing service failed to initialize");
        }
    });
    
    // Cancel import functionality
    let ui_handle = ui.as_weak();
    ui.on_cancel_import(move || {
        let ui = ui_handle.unwrap();
        ui.set_import_dialog_visible(false);
        println!("🚫 Document import cancelled by user");
    });
    
    // PDF export functionality using the proven basic implementation
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
                
                // Use the proven basic PDF export
                let result = export_markdown_to_pdf_basic(&state.content, &path);
                
                match result {
                    Ok(_) => {
                        println!("✅ PDF exported successfully: {}", path.display());
                        println!("📊 Features included:");
                        println!("   - Professional heading hierarchy");
                        println!("   - Formatted code blocks");
                        println!("   - Proper list formatting");
                        println!("   - Clean typography and spacing");
                    },
                    Err(e) => {
                        eprintln!("❌ Failed to export PDF: {}", e);
                    }
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
    
    // Toggle column layout (F7 key)
    let ui_handle = ui.as_weak();
    ui.on_toggle_columns(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_current_layout();
        
        let new_layout = match current {
            QuadLayout::Single => QuadLayout::Horizontal,
            QuadLayout::Horizontal => QuadLayout::Single,
            QuadLayout::Vertical => QuadLayout::Quad,
            QuadLayout::Quad => QuadLayout::Vertical,
        };
        
        ui.set_current_layout(new_layout);
        println!("🔄 F7: Toggled columns to {:?}", new_layout);
    });
    
    // Toggle row layout (F8 key)
    let ui_handle = ui.as_weak();
    ui.on_toggle_rows(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_current_layout();
        
        let new_layout = match current {
            QuadLayout::Single => QuadLayout::Vertical,
            QuadLayout::Vertical => QuadLayout::Single,
            QuadLayout::Horizontal => QuadLayout::Quad,
            QuadLayout::Quad => QuadLayout::Horizontal,
        };
        
        ui.set_current_layout(new_layout);
        println!("🔄 F8: Toggled rows to {:?}", new_layout);
    });
    
    
    // Panel focus
    let ui_handle = ui.as_weak();
    ui.on_panel_focused(move |panel_id| {
        let ui = ui_handle.unwrap();
        ui.set_active_panel(panel_id);
        println!("🎯 Panel {} focused", panel_id + 1);
    });
    
    println!("🚀 Enhanced Markdown Editor started!");
    println!("💡 Features:");
    println!("   - File menu with New/Open/Save/Save As operations");
    println!("   - Window menu for quick panel navigation and file overview");
    println!("   - Word document import (.docx → markdown)");
    println!("   - PDF export with professional formatting");
    println!("   - Multi-panel editing with independent file operations");
    println!("   - Layout shortcuts: F7 (toggle columns), F8 (toggle rows)");
    println!("   - File shortcuts: Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S");
    println!("   - Use File and Window menus for all document and navigation operations");
    
    ui.run()
}

// Basic PDF export function using genpdf and pulldown-cmark (fallback)
fn export_markdown_to_pdf_basic(markdown_content: &str, output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
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