use crate::models::translation_models::{ChunkMetadata, ChunkType, ValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for chunk processing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    pub strategy: ChunkingStrategy,
    pub min_chunk_length: usize,
    pub max_chunk_length: usize,
    pub preserve_formatting: bool,
    pub merge_short_chunks: bool,
    pub custom_delimiters: Vec<String>,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkingStrategy::Sentence,
            min_chunk_length: 10,
            max_chunk_length: 500,
            preserve_formatting: true,
            merge_short_chunks: true,
            custom_delimiters: vec![".".to_string(), "!".to_string(), "?".to_string()],
        }
    }
}

/// Different chunking strategies available
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkingStrategy {
    /// Split by sentence boundaries
    Sentence,
    /// Split by paragraph boundaries
    Paragraph,
    /// Custom chunking with user-defined rules
    Custom {
        delimiters: Vec<String>,
        max_length: usize,
    },
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Represents a processed chunk with its content and metadata
#[derive(Debug, Clone)]
pub struct ProcessedChunk {
    pub content: String,
    pub chunk_type: ChunkType,
    pub original_position: usize,
    pub sentence_boundaries: Vec<usize>,
    pub processing_notes: Vec<String>,
}

/// Service for processing text content into chunks for translation memory
pub struct ChunkProcessor {
    config: ChunkingConfig,
}

impl ChunkProcessor {
    /// Create a new chunk processor with default configuration
    pub fn new() -> Self {
        Self {
            config: ChunkingConfig::default(),
        }
    }

    /// Create a new chunk processor with custom configuration
    pub fn with_config(config: ChunkingConfig) -> Self {
        Self { config }
    }

    /// Update the chunking configuration
    pub fn update_config(&mut self, config: ChunkingConfig) {
        self.config = config;
    }

    /// Process text content into chunks based on the configured strategy
    pub fn process_content(&self, content: &str) -> Result<Vec<ProcessedChunk>, ValidationError> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        match &self.config.strategy {
            ChunkingStrategy::Sentence => self.process_by_sentences(content),
            ChunkingStrategy::Paragraph => self.process_by_paragraphs(content),
            ChunkingStrategy::Custom { delimiters, max_length } => {
                self.process_by_custom_rules(content, delimiters, *max_length)
            }
            ChunkingStrategy::Hybrid => self.process_by_hybrid_strategy(content),
        }
    }

    /// Convert processed chunks to ChunkMetadata for storage
    pub fn chunks_to_metadata(&self, chunks: Vec<ProcessedChunk>) -> Vec<ChunkMetadata> {
        chunks
            .into_iter()
            .map(|chunk| ChunkMetadata {
                id: Uuid::new_v4(),
                original_position: chunk.original_position,
                sentence_boundaries: chunk.sentence_boundaries,
                linked_chunks: Vec::new(),
                chunk_type: chunk.chunk_type,
                processing_notes: chunk.processing_notes,
            })
            .collect()
    }

    /// Reconstruct document content from chunks
    pub fn reconstruct_document(&self, chunks: &[ChunkMetadata], chunk_content: &HashMap<Uuid, String>) -> Result<String, ValidationError> {
        if chunks.is_empty() {
            return Ok(String::new());
        }

        // Sort chunks by original position
        let mut sorted_chunks: Vec<_> = chunks.iter().collect();
        sorted_chunks.sort_by_key(|chunk| chunk.original_position);

        let mut reconstructed = String::new();
        let mut last_position = 0;

        for chunk in sorted_chunks {
            if let Some(content) = chunk_content.get(&chunk.id) {
                // Add spacing if there's a gap in positions
                if chunk.original_position > last_position + 1 {
                    reconstructed.push('\n');
                }

                // Add the chunk content
                reconstructed.push_str(content);

                // Add appropriate spacing based on chunk type
                match chunk.chunk_type {
                    ChunkType::Paragraph => reconstructed.push_str("\n\n"),
                    ChunkType::Heading => reconstructed.push_str("\n\n"),
                    ChunkType::Sentence => {
                        if !content.ends_with('.') && !content.ends_with('!') && !content.ends_with('?') {
                            reconstructed.push('.');
                        }
                        reconstructed.push(' ');
                    }
                    ChunkType::ListItem => reconstructed.push('\n'),
                    ChunkType::CodeBlock => reconstructed.push_str("\n\n"),
                    ChunkType::Table => reconstructed.push_str("\n\n"),
                    _ => reconstructed.push(' '),
                }

                last_position = chunk.original_position;
            }
        }

        // Clean up extra whitespace
        let reconstructed = reconstructed.trim().to_string();
        Ok(reconstructed)
    }

    /// Process content by sentence boundaries
    fn process_by_sentences(&self, content: &str) -> Result<Vec<ProcessedChunk>, ValidationError> {
        let mut chunks = Vec::new();
        let mut position = 0;

        // Split content into paragraphs first to preserve structure
        for paragraph in content.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            // Determine if this is a special paragraph type
            let paragraph_type = self.determine_paragraph_type(paragraph);
            
            match paragraph_type {
                ChunkType::Heading | ChunkType::CodeBlock | ChunkType::Table => {
                    // Keep these as single chunks
                    chunks.push(ProcessedChunk {
                        content: paragraph.to_string(),
                        chunk_type: paragraph_type,
                        original_position: position,
                        sentence_boundaries: vec![0, paragraph.len()],
                        processing_notes: vec!["Special paragraph type preserved".to_string()],
                    });
                    position += 1;
                }
                _ => {
                    // Split paragraph into sentences
                    let sentences = self.split_into_sentences(paragraph);
                    for sentence in sentences {
                        if sentence.trim().len() >= self.config.min_chunk_length {
                            let boundaries = self.detect_sentence_boundaries(&sentence);
                            chunks.push(ProcessedChunk {
                                content: sentence.trim().to_string(),
                                chunk_type: ChunkType::Sentence,
                                original_position: position,
                                sentence_boundaries: boundaries,
                                processing_notes: vec!["Sentence-level chunking".to_string()],
                            });
                            position += 1;
                        }
                    }
                }
            }
        }

        // Merge short chunks if configured
        if self.config.merge_short_chunks {
            chunks = self.merge_short_chunks(chunks)?;
        }

        Ok(chunks)
    }

    /// Process content by paragraph boundaries
    fn process_by_paragraphs(&self, content: &str) -> Result<Vec<ProcessedChunk>, ValidationError> {
        let mut chunks = Vec::new();
        let mut position = 0;

        for paragraph in content.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            if paragraph.len() >= self.config.min_chunk_length {
                let paragraph_type = self.determine_paragraph_type(paragraph);
                let boundaries = self.detect_sentence_boundaries(paragraph);
                
                chunks.push(ProcessedChunk {
                    content: paragraph.to_string(),
                    chunk_type: paragraph_type,
                    original_position: position,
                    sentence_boundaries: boundaries,
                    processing_notes: vec!["Paragraph-level chunking".to_string()],
                });
                position += 1;
            }
        }

        Ok(chunks)
    }

    /// Process content using custom rules
    fn process_by_custom_rules(
        &self,
        content: &str,
        delimiters: &[String],
        max_length: usize,
    ) -> Result<Vec<ProcessedChunk>, ValidationError> {
        let mut chunks = Vec::new();
        let mut position = 0;
        let mut current_chunk = String::new();
        let mut current_boundaries = Vec::new();

        // Create a regex pattern from delimiters
        let _delimiter_pattern = delimiters.join("|");
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];
            current_chunk.push(ch);

            // Check if we hit a delimiter
            let mut found_delimiter = false;
            for delimiter in delimiters {
                if content[i..].starts_with(delimiter) {
                    found_delimiter = true;
                    current_boundaries.push(current_chunk.len());
                    
                    // If chunk is long enough or we've hit max length, create a chunk
                    if current_chunk.trim().len() >= self.config.min_chunk_length 
                        || current_chunk.len() >= max_length {
                        
                        chunks.push(ProcessedChunk {
                            content: current_chunk.trim().to_string(),
                            chunk_type: self.determine_chunk_type(&current_chunk),
                            original_position: position,
                            sentence_boundaries: current_boundaries.clone(),
                            processing_notes: vec!["Custom delimiter chunking".to_string()],
                        });
                        
                        position += 1;
                        current_chunk.clear();
                        current_boundaries.clear();
                    }
                    break;
                }
            }

            // Force split if we exceed max length
            if !found_delimiter && current_chunk.len() >= max_length {
                chunks.push(ProcessedChunk {
                    content: current_chunk.trim().to_string(),
                    chunk_type: self.determine_chunk_type(&current_chunk),
                    original_position: position,
                    sentence_boundaries: current_boundaries.clone(),
                    processing_notes: vec!["Max length chunking".to_string()],
                });
                
                position += 1;
                current_chunk.clear();
                current_boundaries.clear();
            }

            i += 1;
        }

        // Add remaining content as final chunk
        if !current_chunk.trim().is_empty() && current_chunk.trim().len() >= self.config.min_chunk_length {
            chunks.push(ProcessedChunk {
                content: current_chunk.trim().to_string(),
                chunk_type: self.determine_chunk_type(&current_chunk),
                original_position: position,
                sentence_boundaries: current_boundaries,
                processing_notes: vec!["Final chunk".to_string()],
            });
        }

        Ok(chunks)
    }

    /// Process content using hybrid strategy (combines multiple approaches)
    fn process_by_hybrid_strategy(&self, content: &str) -> Result<Vec<ProcessedChunk>, ValidationError> {
        let mut chunks = Vec::new();
        let mut position = 0;

        // First pass: identify special content types
        for paragraph in content.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            let paragraph_type = self.determine_paragraph_type(paragraph);
            
            match paragraph_type {
                ChunkType::Heading | ChunkType::CodeBlock | ChunkType::Table => {
                    // Keep special types as single chunks
                    chunks.push(ProcessedChunk {
                        content: paragraph.to_string(),
                        chunk_type: paragraph_type,
                        original_position: position,
                        sentence_boundaries: vec![0, paragraph.len()],
                        processing_notes: vec!["Hybrid: Special type preserved".to_string()],
                    });
                    position += 1;
                }
                ChunkType::ListItem => {
                    // Process list items individually
                    for line in paragraph.lines() {
                        let line = line.trim();
                        if line.len() >= self.config.min_chunk_length {
                            chunks.push(ProcessedChunk {
                                content: line.to_string(),
                                chunk_type: ChunkType::ListItem,
                                original_position: position,
                                sentence_boundaries: vec![0, line.len()],
                                processing_notes: vec!["Hybrid: List item".to_string()],
                            });
                            position += 1;
                        }
                    }
                }
                _ => {
                    // For regular paragraphs, use sentence-based chunking
                    let sentences = self.split_into_sentences(paragraph);
                    for sentence in sentences {
                        if sentence.trim().len() >= self.config.min_chunk_length {
                            let boundaries = self.detect_sentence_boundaries(&sentence);
                            chunks.push(ProcessedChunk {
                                content: sentence.trim().to_string(),
                                chunk_type: ChunkType::Sentence,
                                original_position: position,
                                sentence_boundaries: boundaries,
                                processing_notes: vec!["Hybrid: Sentence chunking".to_string()],
                            });
                            position += 1;
                        }
                    }
                }
            }
        }

        // Post-process: merge short chunks if configured
        if self.config.merge_short_chunks {
            chunks = self.merge_short_chunks(chunks)?;
        }

        Ok(chunks)
    }

    /// Split text into sentences using improved sentence boundary detection
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];
            current_sentence.push(ch);

            // Check for sentence-ending punctuation
            if matches!(ch, '.' | '!' | '?') {
                // Look ahead to see if this is really a sentence boundary
                if self.is_sentence_boundary(&chars, i) {
                    sentences.push(current_sentence.trim().to_string());
                    current_sentence.clear();
                    
                    // Skip whitespace after sentence boundary
                    while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                        i += 1;
                    }
                }
            }

            i += 1;
        }

        // Add remaining content as final sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }

        // Filter out very short sentences unless they're meaningful
        sentences
            .into_iter()
            .filter(|s| s.len() >= 3 || self.is_meaningful_short_sentence(s))
            .collect()
    }

    /// Determine if a position is a true sentence boundary
    fn is_sentence_boundary(&self, chars: &[char], pos: usize) -> bool {
        if pos >= chars.len() {
            return true;
        }

        let ch = chars[pos];
        
        // Check for abbreviations (simple heuristic)
        if ch == '.' {
            // Look back for common abbreviations
            if pos >= 2 {
                let prev_chars: String = chars[pos.saturating_sub(3)..pos].iter().collect();
                let common_abbrevs = ["Mr.", "Mrs.", "Dr.", "Prof.", "Inc.", "Ltd.", "etc.", "vs.", "e.g.", "i.e."];
                for abbrev in &common_abbrevs {
                    if prev_chars.ends_with(&abbrev[..abbrev.len()-1]) {
                        return false;
                    }
                }
            }

            // Check if followed by lowercase (likely not sentence boundary)
            if pos + 1 < chars.len() {
                let next_non_space = chars[pos + 1..].iter().find(|&&c| !c.is_whitespace());
                if let Some(&next_char) = next_non_space {
                    if next_char.is_lowercase() {
                        return false;
                    }
                }
            }
        }

        // Check if followed by whitespace and capital letter (strong indicator)
        if pos + 1 < chars.len() {
            let mut j = pos + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j].is_uppercase() || chars[j].is_numeric()) {
                return true;
            }
        }

        // End of text is always a boundary
        pos == chars.len() - 1
    }

    /// Check if a short sentence is meaningful (like "Yes." or "No.")
    fn is_meaningful_short_sentence(&self, sentence: &str) -> bool {
        let meaningful_short = ["Yes.", "No.", "OK.", "Hi.", "Bye.", "Thanks.", "Please."];
        meaningful_short.contains(&sentence)
    }

    /// Detect sentence boundaries within a chunk of text
    fn detect_sentence_boundaries(&self, text: &str) -> Vec<usize> {
        let mut boundaries = vec![0]; // Start boundary
        let chars: Vec<char> = text.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            if matches!(ch, '.' | '!' | '?') && self.is_sentence_boundary(&chars, i) {
                boundaries.push(i + 1);
            }
        }

        // Add end boundary if not already present
        if boundaries.last() != Some(&text.len()) {
            boundaries.push(text.len());
        }

        boundaries
    }

    /// Determine the type of a paragraph
    fn determine_paragraph_type(&self, paragraph: &str) -> ChunkType {
        let trimmed = paragraph.trim();
        
        if trimmed.starts_with('#') {
            ChunkType::Heading
        } else if trimmed.starts_with("```") || trimmed.contains("```") {
            ChunkType::CodeBlock
        } else if trimmed.starts_with('|') && trimmed.contains('|') {
            ChunkType::Table
        } else if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
            ChunkType::ListItem
        } else if trimmed.starts_with('`') && trimmed.ends_with('`') {
            ChunkType::Code
        } else if trimmed.starts_with('[') && trimmed.contains("](") {
            ChunkType::Link
        } else {
            ChunkType::Paragraph
        }
    }

    /// Determine the type of a chunk based on its content
    fn determine_chunk_type(&self, content: &str) -> ChunkType {
        let trimmed = content.trim();
        
        // Check for various markdown patterns
        if trimmed.starts_with('#') {
            ChunkType::Heading
        } else if trimmed.starts_with("```") || (trimmed.contains("```") && trimmed.lines().count() > 1) {
            ChunkType::CodeBlock
        } else if trimmed.starts_with('|') && trimmed.contains('|') {
            ChunkType::Table
        } else if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
            ChunkType::ListItem
        } else if trimmed.starts_with('`') && trimmed.ends_with('`') && !trimmed.contains('\n') {
            ChunkType::Code
        } else if trimmed.starts_with('[') && trimmed.contains("](") {
            ChunkType::Link
        } else if trimmed.contains('\n') {
            ChunkType::Paragraph
        } else {
            ChunkType::Sentence
        }
    }

    /// Merge chunks that are too short with adjacent chunks
    fn merge_short_chunks(&self, chunks: Vec<ProcessedChunk>) -> Result<Vec<ProcessedChunk>, ValidationError> {
        if chunks.is_empty() {
            return Ok(chunks);
        }

        let mut merged_chunks = Vec::new();
        let mut current_chunk: Option<ProcessedChunk> = None;

        for chunk in chunks {
            match current_chunk.take() {
                None => {
                    current_chunk = Some(chunk);
                }
                Some(mut prev_chunk) => {
                    // Check if previous chunk is too short and can be merged
                    if prev_chunk.content.len() < self.config.min_chunk_length 
                        && chunk.chunk_type == prev_chunk.chunk_type
                        && chunk.chunk_type.can_be_linked() {
                        
                        // Merge chunks
                        prev_chunk.content.push(' ');
                        prev_chunk.content.push_str(&chunk.content);
                        prev_chunk.sentence_boundaries.extend(
                            chunk.sentence_boundaries.iter().map(|&b| b + prev_chunk.content.len())
                        );
                        prev_chunk.processing_notes.push("Merged with short chunk".to_string());
                        prev_chunk.processing_notes.extend(chunk.processing_notes);
                        
                        current_chunk = Some(prev_chunk);
                    } else {
                        // Keep previous chunk and start new one
                        merged_chunks.push(prev_chunk);
                        current_chunk = Some(chunk);
                    }
                }
            }
        }

        // Add the last chunk
        if let Some(chunk) = current_chunk {
            merged_chunks.push(chunk);
        }

        Ok(merged_chunks)
    }

    /// Get statistics about the chunking process
    pub fn get_chunking_stats(&self, chunks: &[ProcessedChunk]) -> ChunkingStats {
        let mut stats = ChunkingStats::default();
        
        stats.total_chunks = chunks.len();
        
        for chunk in chunks {
            stats.total_characters += chunk.content.len();
            stats.total_words += chunk.content.split_whitespace().count();
            
            match chunk.chunk_type {
                ChunkType::Sentence => stats.sentence_chunks += 1,
                ChunkType::Paragraph => stats.paragraph_chunks += 1,
                ChunkType::Heading => stats.heading_chunks += 1,
                ChunkType::ListItem => stats.list_item_chunks += 1,
                ChunkType::CodeBlock => stats.code_block_chunks += 1,
                ChunkType::Table => stats.table_chunks += 1,
                ChunkType::Code => stats.inline_code_chunks += 1,
                ChunkType::Link => stats.link_chunks += 1,
                ChunkType::LinkedPhrase => stats.linked_phrase_chunks += 1,
            }
            
            if chunk.content.len() < self.config.min_chunk_length {
                stats.short_chunks += 1;
            }
            
            if chunk.content.len() > self.config.max_chunk_length {
                stats.long_chunks += 1;
            }
        }
        
        if !chunks.is_empty() {
            stats.average_chunk_length = stats.total_characters as f32 / chunks.len() as f32;
            stats.average_words_per_chunk = stats.total_words as f32 / chunks.len() as f32;
        }
        
        stats
    }
}

impl Default for ChunkProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the chunking process
#[derive(Debug, Clone, Default)]
pub struct ChunkingStats {
    pub total_chunks: usize,
    pub total_characters: usize,
    pub total_words: usize,
    pub average_chunk_length: f32,
    pub average_words_per_chunk: f32,
    pub sentence_chunks: usize,
    pub paragraph_chunks: usize,
    pub heading_chunks: usize,
    pub list_item_chunks: usize,
    pub code_block_chunks: usize,
    pub table_chunks: usize,
    pub inline_code_chunks: usize,
    pub link_chunks: usize,
    pub linked_phrase_chunks: usize,
    pub short_chunks: usize,
    pub long_chunks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_chunking() {
        let processor = ChunkProcessor::new();
        let content = "This is the first sentence. This is the second sentence! Is this the third sentence?";
        
        let chunks = processor.process_content(content).unwrap();
        
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "This is the first sentence.");
        assert_eq!(chunks[1].content, "This is the second sentence!");
        assert_eq!(chunks[2].content, "Is this the third sentence?");
        
        for chunk in &chunks {
            assert_eq!(chunk.chunk_type, ChunkType::Sentence);
        }
    }

    #[test]
    fn test_paragraph_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Paragraph,
            ..Default::default()
        };
        let processor = ChunkProcessor::with_config(config);
        
        let content = "This is the first paragraph.\n\nThis is the second paragraph with multiple sentences. It has more content.";
        
        let chunks = processor.process_content(content).unwrap();
        
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].content, "This is the first paragraph.");
        assert!(chunks[1].content.contains("This is the second paragraph"));
    }

    #[test]
    fn test_custom_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Custom {
                delimiters: vec![";".to_string(), ":".to_string()],
                max_length: 50,
            },
            ..Default::default()
        };
        let processor = ChunkProcessor::with_config(config);
        
        let content = "First part; Second part: Third part";
        
        let chunks = processor.process_content(content).unwrap();
        
        assert!(chunks.len() >= 2);
        assert!(chunks[0].content.contains("First part"));
    }

    #[test]
    fn test_hybrid_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Hybrid,
            ..Default::default()
        };
        let processor = ChunkProcessor::with_config(config);
        
        let content = "# Heading\n\nThis is a paragraph. With multiple sentences.\n\n- List item 1\n- List item 2\n\n```\ncode block\n```";
        
        let chunks = processor.process_content(content).unwrap();
        
        // Should have different chunk types
        let chunk_types: Vec<_> = chunks.iter().map(|c| &c.chunk_type).collect();
        assert!(chunk_types.contains(&&ChunkType::Heading));
        assert!(chunk_types.contains(&&ChunkType::Sentence));
        assert!(chunk_types.contains(&&ChunkType::ListItem));
        assert!(chunk_types.contains(&&ChunkType::CodeBlock));
    }

    #[test]
    fn test_sentence_boundary_detection() {
        let processor = ChunkProcessor::new();
        
        // Test abbreviations
        let text = "Dr. Smith went to the store. He bought milk.";
        let boundaries = processor.detect_sentence_boundaries(text);
        
        // Should not split on "Dr."
        assert!(boundaries.len() >= 2);
        
        // Test with numbers
        let text2 = "The price is $5.99. That's expensive.";
        let boundaries2 = processor.detect_sentence_boundaries(text2);
        
        // Should not split on "5.99"
        assert!(boundaries2.len() >= 2);
    }

    #[test]
    fn test_chunk_reconstruction() {
        let processor = ChunkProcessor::new();
        let content = "First sentence. Second sentence.";
        
        let processed_chunks = processor.process_content(content).unwrap();
        let chunks = processor.chunks_to_metadata(processed_chunks.clone());
        
        // Create content map
        let mut chunk_content = HashMap::new();
        for (i, chunk) in chunks.iter().enumerate() {
            chunk_content.insert(chunk.id, processed_chunks[i].content.clone());
        }
        
        let reconstructed = processor.reconstruct_document(&chunks, &chunk_content).unwrap();
        
        // Should be similar to original (may have slight formatting differences)
        assert!(reconstructed.contains("First sentence"));
        assert!(reconstructed.contains("Second sentence"));
    }

    #[test]
    fn test_chunk_type_detection() {
        let processor = ChunkProcessor::new();
        
        assert_eq!(processor.determine_chunk_type("# Heading"), ChunkType::Heading);
        assert_eq!(processor.determine_chunk_type("```code```"), ChunkType::CodeBlock);
        assert_eq!(processor.determine_chunk_type("- List item"), ChunkType::ListItem);
        assert_eq!(processor.determine_chunk_type("`inline code`"), ChunkType::Code);
        assert_eq!(processor.determine_chunk_type("[link](url)"), ChunkType::Link);
        assert_eq!(processor.determine_chunk_type("| table | cell |"), ChunkType::Table);
        assert_eq!(processor.determine_chunk_type("Regular sentence."), ChunkType::Sentence);
    }

    #[test]
    fn test_chunking_stats() {
        let processor = ChunkProcessor::new();
        let content = "# Heading\n\nFirst sentence. Second sentence.\n\n- List item";
        
        let chunks = processor.process_content(content).unwrap();
        let stats = processor.get_chunking_stats(&chunks);
        
        assert!(stats.total_chunks > 0);
        assert!(stats.total_characters > 0);
        assert!(stats.total_words > 0);
        assert!(stats.heading_chunks > 0);
        assert!(stats.sentence_chunks > 0);
        assert!(stats.list_item_chunks > 0);
    }

    #[test]
    fn test_merge_short_chunks() {
        let config = ChunkingConfig {
            min_chunk_length: 20,
            merge_short_chunks: true,
            ..Default::default()
        };
        let processor = ChunkProcessor::with_config(config);
        
        let content = "Short. Another short. This is a longer sentence that meets the minimum length requirement.";
        
        let chunks = processor.process_content(content).unwrap();
        
        // Short chunks should be merged
        assert!(chunks.iter().any(|c| c.content.len() >= 20));
    }
}