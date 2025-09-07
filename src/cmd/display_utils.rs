//! Display utilities for consistent styling and layout in terminal interfaces.
//! 
//! This module provides functions for:
//! - Chinese/English character width calculation
//! - Text alignment and padding
//! - Terminal width detection and adaptive layout
//! - Consistent formatting for configuration display


/// Calculate the display width of a string considering Chinese/English character differences.
/// 
/// Chinese characters typically take 2 terminal columns while ASCII characters take 1.
/// This function provides accurate width calculation for mixed Chinese/English text.
/// 
/// # Arguments
/// * `text` - The text to measure
/// 
/// # Returns
/// The display width in terminal columns
/// 
/// # Examples
/// ```
/// use cc_switch::cmd::display_utils::text_display_width;
/// 
/// assert_eq!(text_display_width("Hello"), 5);           // 5 ASCII chars = 5 columns
/// assert_eq!(text_display_width("你好"), 4);              // 2 Chinese chars = 4 columns  
/// assert_eq!(text_display_width("Hello你好"), 9);         // 5 ASCII + 2 Chinese = 9 columns
/// ```
pub fn text_display_width(text: &str) -> usize {
    text.chars()
        .map(|c| {
            // Check if character is likely a wide character (Chinese, Japanese, Korean, etc.)
            // Using Unicode properties to detect wide characters
            match c as u32 {
                // ASCII range: 1 column
                0x00..=0x7F => 1,
                // Latin extended, symbols: mostly 1 column
                0x80..=0x2FF => 1,
                // CJK symbols and punctuation: 2 columns
                0x3000..=0x303F => 2,
                // Hiragana: 2 columns
                0x3040..=0x309F => 2,
                // Katakana: 2 columns
                0x30A0..=0x30FF => 2,
                // CJK Unified Ideographs: 2 columns
                0x4E00..=0x9FFF => 2,
                // Hangul Syllables: 2 columns
                0xAC00..=0xD7AF => 2,
                // CJK Unified Ideographs Extension A: 2 columns
                0x3400..=0x4DBF => 2,
                // Full-width ASCII: 2 columns
                0xFF01..=0xFF5E => 2,
                // Other characters: assume 1 column (conservative estimate)
                _ => 1,
            }
        })
        .sum()
}

/// Pad text to a specific display width, handling Chinese/English character differences.
/// 
/// # Arguments
/// * `text` - The text to pad
/// * `width` - Target display width in terminal columns
/// * `alignment` - Text alignment (Left, Right, Center)
/// * `pad_char` - Character to use for padding (default: space)
/// 
/// # Returns
/// Padded text string
/// 
/// # Examples
/// ```
/// use cc_switch::cmd::display_utils::{pad_text_to_width, TextAlignment};
/// 
/// assert_eq!(pad_text_to_width("Hello", 10, TextAlignment::Left, ' '), "Hello     ");
/// assert_eq!(pad_text_to_width("你好", 10, TextAlignment::Center, ' '), "   你好   ");
/// ```
pub fn pad_text_to_width(text: &str, width: usize, alignment: TextAlignment, pad_char: char) -> String {
    let text_width = text_display_width(text);
    
    if text_width >= width {
        return text.to_string();
    }
    
    let padding_needed = width - text_width;
    
    match alignment {
        TextAlignment::Left => {
            format!("{}{}", text, pad_char.to_string().repeat(padding_needed))
        }
        TextAlignment::Right => {
            format!("{}{}", pad_char.to_string().repeat(padding_needed), text)
        }
        TextAlignment::Center => {
            let left_pad = padding_needed / 2;
            let right_pad = padding_needed - left_pad;
            format!("{}{}{}", 
                pad_char.to_string().repeat(left_pad),
                text,
                pad_char.to_string().repeat(right_pad)
            )
        }
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Right,
    Center,
}

/// Detect current terminal width, with fallback to default
/// 
/// # Returns
/// Terminal width in columns, defaults to 80 if detection fails
pub fn get_terminal_width() -> usize {
    if let Ok((width, _)) = crossterm::terminal::size() {
        width as usize
    } else {
        80 // Fallback width
    }
}

/// Create a horizontal line for borders and separators
/// 
/// # Arguments
/// * `width` - Line width in terminal columns
/// * `line_char` - Character to use for the line
/// 
/// # Returns
/// String containing the horizontal line
pub fn create_horizontal_line(width: usize, line_char: char) -> String {
    line_char.to_string().repeat(width)
}

/// Format a configuration token for safe display
/// 
/// This is a centralized version of the token formatting logic,
/// ensuring consistent display across the application.
/// 
/// # Arguments
/// * `token` - The API token to format
/// 
/// # Returns
/// Safely formatted token string
pub fn format_token_for_display(token: &str) -> String {
    const PREFIX_LEN: usize = 12;
    const SUFFIX_LEN: usize = 8;
    
    if token.len() <= PREFIX_LEN + SUFFIX_LEN {
        // If token is short enough, show first half and mask the rest with *
        if token.len() <= 6 {
            // Very short token, just show first few chars and mask the rest
            let visible_chars = (token.len() + 1) / 2;
            format!("{}***", &token[..visible_chars])
        } else {
            // Medium length token, show some chars from start and mask the end
            let visible_chars = token.len() / 2;
            format!("{}***", &token[..visible_chars])
        }
    } else {
        // Long token, use the original format: first 12 + "..." + last 8
        format!("{}...{}", &token[..PREFIX_LEN], &token[token.len() - SUFFIX_LEN..])
    }
}

/// Calculate optimal box width for content display
/// 
/// Determines the best width for content boxes based on terminal size
/// and content requirements, with reasonable minimum and maximum limits.
/// 
/// # Arguments
/// * `min_width` - Minimum required width
/// * `max_width` - Maximum allowed width  
/// * `content_width` - Preferred width based on content
/// 
/// # Returns
/// Optimal box width in terminal columns
pub fn calculate_optimal_box_width(min_width: usize, max_width: usize, content_width: usize) -> usize {
    let terminal_width = get_terminal_width();
    let max_usable_width = if terminal_width > 4 { terminal_width - 4 } else { terminal_width };
    
    content_width
        .max(min_width)
        .min(max_width)
        .min(max_usable_width)
}

/// Create a bordered content line with proper padding
/// 
/// # Arguments  
/// * `content` - The content text to display
/// * `total_width` - Total width of the bordered line
/// * `alignment` - Text alignment within the border
/// 
/// # Returns
/// Formatted line with borders and proper padding
pub fn create_bordered_line(content: &str, total_width: usize, alignment: TextAlignment) -> String {
    if total_width < 4 {
        return content.to_string();
    }
    
    let inner_width = total_width - 4; // Account for "║ " and " ║"
    let padded_content = pad_text_to_width(content, inner_width, alignment, ' ');
    
    format!("║ {} ║", padded_content)
}

/// Layout configuration for displaying configuration items
#[derive(Debug, Clone)]
pub struct ConfigDisplayLayout {
    pub box_width: usize,
    pub content_width: usize,
    pub indent: String,
    pub item_spacing: usize,
}

impl ConfigDisplayLayout {
    /// Create a new layout optimized for current terminal
    /// 
    /// # Arguments
    /// * `min_width` - Minimum box width required
    /// 
    /// # Returns
    /// Optimized layout configuration
    pub fn new(min_width: usize) -> Self {
        let box_width = calculate_optimal_box_width(min_width, 80, 60);
        let content_width = if box_width > 4 { box_width - 4 } else { box_width };
        
        Self {
            box_width,
            content_width,
            indent: "  ".to_string(),
            item_spacing: 1,
        }
    }
    
    /// Create box header with title
    pub fn create_header(&self, title: &str) -> String {
        let top_border = format!("╔{}╗", "═".repeat(self.box_width - 2));
        let title_line = create_bordered_line(title, self.box_width, TextAlignment::Center);
        let bottom_border = format!("╚{}╝", "═".repeat(self.box_width - 2));
        
        format!("{}\n{}\n{}", top_border, title_line, bottom_border)
    }
    
    /// Create instruction line
    pub fn create_instruction(&self, text: &str) -> String {
        create_bordered_line(text, self.box_width, TextAlignment::Left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_display_width() {
        // ASCII characters
        assert_eq!(text_display_width("Hello"), 5);
        assert_eq!(text_display_width("World"), 5);
        assert_eq!(text_display_width("123"), 3);
        
        // Chinese characters (should be 2 columns each)
        assert_eq!(text_display_width("你好"), 4);
        assert_eq!(text_display_width("测试"), 4);
        
        // Mixed Chinese and English
        assert_eq!(text_display_width("Hello你好"), 9); // 5 + 4
        assert_eq!(text_display_width("测试123"), 7);   // 4 + 3
        
        // Empty string
        assert_eq!(text_display_width(""), 0);
        
        // Special characters and punctuation
        assert_eq!(text_display_width("!@#$%"), 5);
        
        // Full-width punctuation (should be 2 columns each)
        assert_eq!(text_display_width("！（）"), 6);
    }

    #[test]
    fn test_pad_text_to_width() {
        // Left alignment
        assert_eq!(pad_text_to_width("Hello", 10, TextAlignment::Left, ' '), "Hello     ");
        
        // Right alignment
        assert_eq!(pad_text_to_width("Hello", 10, TextAlignment::Right, ' '), "     Hello");
        
        // Center alignment
        assert_eq!(pad_text_to_width("Hi", 6, TextAlignment::Center, ' '), "  Hi  ");
        assert_eq!(pad_text_to_width("Hi", 7, TextAlignment::Center, ' '), "  Hi   ");
        
        // Text wider than target width (should return original)
        assert_eq!(pad_text_to_width("Very long text", 5, TextAlignment::Left, ' '), "Very long text");
        
        // Chinese characters
        assert_eq!(pad_text_to_width("你好", 8, TextAlignment::Left, ' '), "你好    ");
        assert_eq!(pad_text_to_width("你好", 8, TextAlignment::Center, ' '), "  你好  ");
        
        // Different padding characters
        assert_eq!(pad_text_to_width("test", 8, TextAlignment::Center, '-'), "--test--");
    }

    #[test]
    fn test_format_token_for_display() {
        // Very short token (3 chars: (3+1)/2 = 2 chars visible)
        assert_eq!(format_token_for_display("abc"), "ab***");
        // 6 chars: 6/2 = 3 chars visible
        assert_eq!(format_token_for_display("abcdef"), "abc***");
        
        // Medium length token
        assert_eq!(format_token_for_display("abcdefgh"), "abcd***");
        
        // Long token (standard format)
        let long_token = "sk-ant-api03_abcdefghijklmnopqrstuvwxyz1234567890abcdefgh";
        let formatted = format_token_for_display(long_token);
        assert!(formatted.starts_with("sk-ant-api03"));
        assert!(formatted.contains("..."));
        assert!(formatted.ends_with("defgh"));
        assert_eq!(formatted.len(), 12 + 3 + 8); // prefix + "..." + suffix
    }

    #[test]
    fn test_calculate_optimal_box_width() {
        // Normal case
        assert_eq!(calculate_optimal_box_width(30, 100, 50), 50);
        
        // Content width too small
        assert_eq!(calculate_optimal_box_width(30, 100, 20), 30);
        
        // Content width too large
        assert_eq!(calculate_optimal_box_width(30, 60, 80), 60);
    }

    #[test]
    fn test_create_bordered_line() {
        // Normal case
        assert_eq!(create_bordered_line("Test", 10, TextAlignment::Left), "║ Test   ║");
        assert_eq!(create_bordered_line("Test", 10, TextAlignment::Center), "║  Test  ║");
        assert_eq!(create_bordered_line("Test", 10, TextAlignment::Right), "║   Test ║");
        
        // Too narrow (should return content as-is)
        assert_eq!(create_bordered_line("Test", 3, TextAlignment::Left), "Test");
        
        // Chinese characters
        assert_eq!(create_bordered_line("你好", 10, TextAlignment::Left), "║ 你好   ║");
    }

    #[test]
    fn test_config_display_layout() {
        let layout = ConfigDisplayLayout::new(40);
        
        // Should have reasonable dimensions
        assert!(layout.box_width >= 40);
        assert_eq!(layout.content_width, layout.box_width - 4);
        
        // Test header creation
        let header = layout.create_header("Test Header");
        assert!(header.contains("Test Header"));
        assert!(header.contains("╔"));
        assert!(header.contains("╗"));
        assert!(header.contains("╚"));
        assert!(header.contains("╝"));
        
        // Test instruction creation
        let instruction = layout.create_instruction("Press Enter to continue");
        assert!(instruction.contains("Press Enter to continue"));
        assert!(instruction.starts_with("║"));
        assert!(instruction.ends_with("║"));
    }

    #[test]
    fn test_create_horizontal_line() {
        assert_eq!(create_horizontal_line(5, '='), "=====");
        assert_eq!(create_horizontal_line(0, '='), "");
        assert_eq!(create_horizontal_line(3, '─'), "───");
    }
}