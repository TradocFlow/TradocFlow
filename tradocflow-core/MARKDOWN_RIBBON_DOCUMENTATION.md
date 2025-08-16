# Markdown Ribbon Interface Documentation

## Overview

The Markdown Ribbon Interface provides a modern, Office-style toolbar for the Tradocument Reviewer markdown editor. It organizes markdown editing tools into logical groups for improved user experience and productivity.

## Features

### 1. **Ribbon Groups**
The ribbon is organized into logical sections:

- **File**: Document operations (New, Open, Save, Export)
- **Clipboard**: Undo/Redo operations
- **Format**: Text formatting (Bold, Italic, Underline, Strikethrough, Code)
- **Structure**: Document structure (Headings, Blockquotes, Code blocks)
- **Lists**: List creation (Bullet, Numbered, Checklist)
- **Insert**: Content insertion (Links, Images, Tables, Horizontal rules)
- **Align**: Text alignment (Left, Center, Right)
- **Indent**: Text indentation controls
- **View**: Layout and preview toggles

### 2. **Visual Design**
- Modern, professional appearance inspired by Office applications
- Hover effects and visual feedback
- Consistent iconography using Unicode symbols
- Accessible design with proper contrast ratios
- Responsive layout that adapts to window size

### 3. **Accessibility Features**
- Keyboard navigation support
- Screen reader friendly labels via tooltips
- High contrast design elements
- Focus indicators for keyboard users
- Semantic structure for assistive technologies

### 4. **Keyboard Shortcuts**
All major functions include keyboard shortcuts:
- **Ctrl+N**: New Document
- **Ctrl+O**: Open Document  
- **Ctrl+S**: Save Document
- **Ctrl+E**: Export Document
- **Ctrl+Z**: Undo
- **Ctrl+Y**: Redo
- **Ctrl+B**: Bold
- **Ctrl+I**: Italic
- **Ctrl+U**: Underline
- **Ctrl+K**: Insert Link
- **Ctrl+R**: Toggle Ribbon

## Architecture

### Component Structure

```
MarkdownRibbon (main component)
├── RibbonGroup (container for related buttons)
│   ├── RibbonButton (standard icon button)
│   ├── RibbonTextButton (button with text label)
│   └── HeadingDropdown (specialized dropdown for headings)
├── RibbonSeparator (visual separator between groups)
└── Various callback handlers
```

### Key Components

#### RibbonButton
```slint
RibbonButton {
    icon: "📄";
    tooltip: "New Document (Ctrl+N)";
    enabled: root.enabled;
    clicked => { root.file-new(); }
}
```

#### RibbonTextButton
```slint
RibbonTextButton {
    text: "Preview";
    icon: "👁";
    tooltip: "Toggle Preview";
    enabled: root.enabled;
    clicked => { root.toggle-preview(); }
}
```

#### HeadingDropdown
```slint
HeadingDropdown {
    tooltip: "Heading Level";
    selected(level) => { root.format-heading(level); }
}
```

#### RibbonGroup
```slint
RibbonGroup {
    title: "Format";
    // Child buttons go here
}
```

## Integration

### MainWindow Integration

The ribbon is integrated into the MainWindow component at `/src/ui/main.slint`:

1. **Import**: Added to imports section
2. **Properties**: Added ribbon state properties
3. **Callbacks**: Added ribbon-specific callbacks  
4. **Layout**: Inserted between MenuBar and main content area
5. **Keyboard**: Added Ctrl+R shortcut for ribbon toggle

### MenuBar Integration

Added "Toggle Ribbon" option to the View menu with Ctrl+R shortcut.

### State Management

The ribbon state is managed through MainWindow properties:
- `show-ribbon`: Controls ribbon visibility
- `can-undo`/`can-redo`: Enable/disable undo/redo buttons
- `has-selection`: For selection-dependent operations
- `enabled`: Global enable/disable state

## Callback Implementation

### Text Formatting Callbacks

Implement these callbacks in your Rust backend:

```rust
impl MainWindowLogic {
    fn format_bold(&mut self) {
        // Wrap selected text with **bold** markdown
        self.wrap_selection("**", "**");
    }
    
    fn format_italic(&mut self) {
        // Wrap selected text with *italic* markdown
        self.wrap_selection("*", "*");
    }
    
    fn format_strikethrough(&mut self) {
        // Wrap selected text with ~~strikethrough~~ markdown
        self.wrap_selection("~~", "~~");
    }
    
    fn format_heading(&mut self, level: i32) {
        // Add heading markdown at current line
        let prefix = "#".repeat(level as usize) + " ";
        self.add_line_prefix(&prefix);
    }
}
```

### List Creation Callbacks

```rust
impl MainWindowLogic {
    fn insert_bullet_list(&mut self) {
        self.add_line_prefix("- ");
    }
    
    fn insert_numbered_list(&mut self) {
        self.add_line_prefix("1. ");
    }
    
    fn insert_checklist(&mut self) {
        self.add_line_prefix("- [ ] ");
    }
}
```

### Insert Operations Callbacks

```rust
impl MainWindowLogic {
    fn insert_link(&mut self) {
        self.insert_text("[Link Text](https://example.com)");
    }
    
    fn insert_image(&mut self) {
        self.insert_text("![Alt Text](image-url)");
    }
    
    fn insert_table(&mut self) {
        let table = "| Column 1 | Column 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        self.insert_text(table);
    }
    
    fn insert_horizontal_rule(&mut self) {
        self.insert_text("\n---\n");
    }
}
```

## Customization Guide

### Adding New Buttons

1. **Define the callback** in MainWindow:
```slint
callback my-custom-action();
```

2. **Add button to ribbon group**:
```slint
RibbonButton {
    icon: "🔧";
    tooltip: "My Custom Action";
    enabled: root.enabled;
    clicked => { root.my-custom-action(); }
}
```

3. **Connect callback in MainWindow**:
```slint
my-custom-action => { root.my-custom-action(); }
```

4. **Implement in Rust backend**:
```rust
fn my_custom_action(&mut self) {
    // Your implementation here
}
```

### Creating New Ribbon Groups

```slint
RibbonSeparator { }

RibbonGroup {
    title: "My Group";
    
    RibbonButton {
        icon: "🎯";
        tooltip: "Action 1";
        clicked => { root.action1(); }
    }
    
    RibbonButton {
        icon: "⚡";
        tooltip: "Action 2";
        clicked => { root.action2(); }
    }
}
```

### Styling Customization

Colors and themes are controlled via the existing theme system:

- **Colors**: `/src/ui/styles/colors.slint`
- **Theme**: `/src/ui/styles/default.slint`

Key customizable properties:
- `Colors.primary`: Primary accent color
- `Colors.surface`: Background colors
- `Colors.text-primary`: Text colors
- `Theme.spacing-*`: Spacing values
- `Theme.border-radius-*`: Border radius values

### Icon Customization

The ribbon uses Unicode symbols for icons. To customize:

1. **Replace icon text** in RibbonButton components
2. **Use emoji or Unicode symbols** for consistency
3. **Consider font support** across platforms

Recommended icon sources:
- Unicode symbols (✓, ✗, ◐, ▶, etc.)
- Emoji (📄, 📁, 💾, 🔗, etc.)
- Mathematical symbols (⊞, ≡, ⇤, ⇥, etc.)

## Performance Considerations

### Efficient Updates

- **State binding**: Ribbon buttons automatically update based on state properties
- **Conditional rendering**: Ribbon only renders when `show-ribbon` is true
- **Event handling**: Callbacks are only invoked when buttons are enabled

### Memory Usage

- **Lightweight components**: Each button is a simple Rectangle with minimal overhead
- **Shared resources**: Colors and themes are global resources
- **No heavy dependencies**: Uses only standard Slint widgets

## Testing

### Manual Testing Checklist

- [ ] All buttons respond to clicks
- [ ] Keyboard shortcuts work correctly
- [ ] Tooltip text displays on hover
- [ ] Ribbon toggles via Ctrl+R and View menu
- [ ] Visual feedback on hover/press
- [ ] Dropdown menus open/close correctly
- [ ] Ribbon adapts to window resizing

### Automated Testing

Consider implementing tests for:
- Callback invocation verification
- State property updates
- Keyboard shortcut handling
- UI responsiveness

## Troubleshooting

### Common Issues

1. **Buttons not responding**
   - Check that callbacks are properly connected in MainWindow
   - Verify `enabled` property is true
   - Ensure Rust backend implements callback handlers

2. **Icons not displaying**
   - Verify Unicode symbol support in system fonts
   - Check font rendering in Slint application
   - Consider using alternative symbols

3. **Layout issues**
   - Check spacing and padding values in theme
   - Verify container sizing in parent components
   - Test with different window sizes

4. **Performance problems**
   - Profile callback execution time
   - Check for excessive property bindings
   - Optimize heavy operations in callbacks

### Debug Tips

- Use Slint's debug mode to inspect component hierarchy
- Log callback invocations in Rust backend
- Test individual components in isolation
- Verify property binding relationships

## Future Enhancements

### Planned Features

1. **Contextual ribbons**: Different ribbon layouts for different editing modes
2. **Custom button groups**: User-configurable button arrangements
3. **Theme variants**: Light/dark theme support
4. **Advanced dropdowns**: More sophisticated dropdown menus
5. **Plugin support**: Third-party ribbon extensions

### Extension Points

The ribbon architecture supports:
- Dynamic button enabling/disabling
- Runtime callback registration
- Theme-based styling
- Accessibility enhancements
- Internationalization

## Conclusion

The Markdown Ribbon Interface provides a comprehensive, accessible, and extensible toolbar for markdown editing. Its modular design allows for easy customization while maintaining professional appearance and functionality. The integration with the existing Slint theme system ensures consistency with the overall application design.

For additional support or feature requests, refer to the project documentation or submit issues through the appropriate channels.