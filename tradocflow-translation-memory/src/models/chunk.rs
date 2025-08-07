//! Chunk models for content processing
//! 
//! Extracted from the original TradocFlow core translation models

// ValidationError not used in this module
use crate::error::{Result, TranslationMemoryError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Chunk metadata for sentence chunking and linking information
/// Extracted from the original TradocFlow core translation models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Unique identifier
    pub id: Uuid,
    
    /// Original position in the document
    pub original_position: usize,
    
    /// Sentence boundaries within the chunk
    pub sentence_boundaries: Vec<usize>,
    
    /// IDs of linked chunks
    pub linked_chunks: Vec<Uuid>,
    
    /// Type of content chunk
    pub chunk_type: ChunkType,
    
    /// Processing notes and metadata
    pub processing_notes: Vec<String>,
}

/// Types of content chunks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkType {
    /// Individual sentence
    Sentence,
    
    /// Complete paragraph
    Paragraph,
    
    /// Section heading
    Heading,
    
    /// List item
    ListItem,
    
    /// Code block
    CodeBlock,
    
    /// Table content
    Table,
    
    /// Linked phrase group
    LinkedPhrase,
    
    /// Inline code
    Code,
    
    /// Link or reference
    Link,
}

impl ChunkMetadata {
    /// Create new chunk metadata with validation
    pub fn new(
        original_position: usize,
        sentence_boundaries: Vec<usize>,
        chunk_type: ChunkType,
    ) -> Result<Self> {
        // Validate sentence boundaries are sorted
        if !sentence_boundaries.windows(2).all(|w| w[0] <= w[1]) {
            return Err(TranslationMemoryError::DataValidation(
                "Sentence boundaries must be sorted".to_string()
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            original_position,
            sentence_boundaries,
            linked_chunks: Vec::new(),
            chunk_type,
            processing_notes: Vec::new(),
        })
    }

    /// Link this chunk with another chunk
    pub fn link_with_chunk(&mut self, chunk_id: Uuid) -> Result<()> {
        if chunk_id == self.id {
            return Err(TranslationMemoryError::DataValidation(
                "Cannot link chunk with itself".to_string()
            ));
        }

        if !self.linked_chunks.contains(&chunk_id) {
            self.linked_chunks.push(chunk_id);
        }

        Ok(())
    }

    /// Unlink this chunk from another chunk
    pub fn unlink_from_chunk(&mut self, chunk_id: &Uuid) {
        self.linked_chunks.retain(|id| id != chunk_id);
    }

    /// Add a processing note
    pub fn add_processing_note(&mut self, note: String) {
        if !note.trim().is_empty() {
            self.processing_notes.push(note);
        }
    }

    /// Check if this chunk is linked to another chunk
    pub fn is_linked_to(&self, chunk_id: &Uuid) -> bool {
        self.linked_chunks.contains(chunk_id)
    }

    /// Get the number of linked chunks
    pub fn linked_chunk_count(&self) -> usize {
        self.linked_chunks.len()
    }

    /// Validate the chunk metadata
    pub fn validate(&self) -> Result<()> {
        // Validate sentence boundaries are sorted
        if !self.sentence_boundaries.windows(2).all(|w| w[0] <= w[1]) {
            return Err(TranslationMemoryError::DataValidation(
                "Sentence boundaries must be sorted".to_string()
            ));
        }

        // Validate no self-links
        if self.linked_chunks.contains(&self.id) {
            return Err(TranslationMemoryError::DataValidation(
                "Chunk cannot be linked to itself".to_string()
            ));
        }

        Ok(())
    }
}

impl ChunkType {
    /// Get all available chunk types
    pub fn all() -> Vec<ChunkType> {
        vec![
            ChunkType::Sentence,
            ChunkType::Paragraph,
            ChunkType::Heading,
            ChunkType::ListItem,
            ChunkType::CodeBlock,
            ChunkType::Table,
            ChunkType::LinkedPhrase,
            ChunkType::Code,
            ChunkType::Link,
        ]
    }

    /// Get chunk type description
    pub fn description(&self) -> &'static str {
        match self {
            ChunkType::Sentence => "Individual sentence",
            ChunkType::Paragraph => "Complete paragraph",
            ChunkType::Heading => "Section heading",
            ChunkType::ListItem => "List item",
            ChunkType::CodeBlock => "Code block",
            ChunkType::Table => "Table content",
            ChunkType::LinkedPhrase => "Linked phrase group",
            ChunkType::Code => "Inline code",
            ChunkType::Link => "Link or reference",
        }
    }

    /// Check if this chunk type can be linked with others
    pub fn can_be_linked(&self) -> bool {
        matches!(self, ChunkType::Sentence | ChunkType::ListItem | ChunkType::LinkedPhrase)
    }
}

/// Builder for chunk metadata
#[derive(Debug, Default)]
pub struct ChunkBuilder {
    original_position: Option<usize>,
    sentence_boundaries: Vec<usize>,
    chunk_type: Option<ChunkType>,
    processing_notes: Vec<String>,
}

impl ChunkBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set original position
    pub fn original_position(mut self, position: usize) -> Self {
        self.original_position = Some(position);
        self
    }
    
    /// Add sentence boundary
    pub fn sentence_boundary(mut self, boundary: usize) -> Self {
        self.sentence_boundaries.push(boundary);
        self
    }
    
    /// Set sentence boundaries
    pub fn sentence_boundaries(mut self, boundaries: Vec<usize>) -> Self {
        self.sentence_boundaries = boundaries;
        self
    }
    
    /// Set chunk type
    pub fn chunk_type(mut self, chunk_type: ChunkType) -> Self {
        self.chunk_type = Some(chunk_type);
        self
    }
    
    /// Add processing note
    pub fn processing_note<S: Into<String>>(mut self, note: S) -> Self {
        self.processing_notes.push(note.into());
        self
    }
    
    /// Build the chunk metadata
    pub fn build(mut self) -> Result<ChunkMetadata> {
        let original_position = self.original_position
            .ok_or_else(|| TranslationMemoryError::DataValidation("Original position is required".to_string()))?;
        let chunk_type = self.chunk_type
            .ok_or_else(|| TranslationMemoryError::DataValidation("Chunk type is required".to_string()))?;
        
        // Sort sentence boundaries
        self.sentence_boundaries.sort_unstable();
        
        let mut metadata = ChunkMetadata::new(original_position, self.sentence_boundaries, chunk_type)?;
        
        // Add processing notes
        for note in self.processing_notes {
            metadata.add_processing_note(note);
        }
        
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_metadata_creation() {
        let chunk = ChunkMetadata::new(
            0,
            vec![0, 12, 18],
            ChunkType::Sentence,
        );

        assert!(chunk.is_ok());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.original_position, 0);
        assert_eq!(chunk.sentence_boundaries, vec![0, 12, 18]);
        assert_eq!(chunk.chunk_type, ChunkType::Sentence);
        assert!(chunk.linked_chunks.is_empty());
        assert!(chunk.processing_notes.is_empty());
    }

    #[test]
    fn test_chunk_metadata_validation_unsorted_boundaries() {
        let result = ChunkMetadata::new(
            0,
            vec![12, 0, 18], // Unsorted boundaries
            ChunkType::Sentence,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_linking() {
        let mut chunk1 = ChunkMetadata::new(0, vec![0, 12], ChunkType::Sentence).unwrap();
        let chunk2_id = Uuid::new_v4();

        let result = chunk1.link_with_chunk(chunk2_id);
        assert!(result.is_ok());
        assert!(chunk1.is_linked_to(&chunk2_id));
        assert_eq!(chunk1.linked_chunk_count(), 1);

        // Test unlinking
        chunk1.unlink_from_chunk(&chunk2_id);
        assert!(!chunk1.is_linked_to(&chunk2_id));
        assert_eq!(chunk1.linked_chunk_count(), 0);
    }

    #[test]
    fn test_chunk_self_linking_prevention() {
        let mut chunk = ChunkMetadata::new(0, vec![0, 12], ChunkType::Sentence).unwrap();
        let result = chunk.link_with_chunk(chunk.id);

        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_type_properties() {
        assert!(ChunkType::Sentence.can_be_linked());
        assert!(ChunkType::ListItem.can_be_linked());
        assert!(ChunkType::LinkedPhrase.can_be_linked());
        assert!(!ChunkType::Heading.can_be_linked());
        assert!(!ChunkType::CodeBlock.can_be_linked());

        assert_eq!(ChunkType::Sentence.description(), "Individual sentence");
        assert_eq!(ChunkType::Paragraph.description(), "Complete paragraph");
    }
    
    #[test]
    fn test_chunk_builder() {
        let chunk = ChunkBuilder::new()
            .original_position(5)
            .sentence_boundary(0)
            .sentence_boundary(10)
            .sentence_boundary(20)
            .chunk_type(ChunkType::Paragraph)
            .processing_note("Test note")
            .build()
            .unwrap();
        
        assert_eq!(chunk.original_position, 5);
        assert_eq!(chunk.sentence_boundaries, vec![0, 10, 20]);
        assert_eq!(chunk.chunk_type, ChunkType::Paragraph);
        assert_eq!(chunk.processing_notes.len(), 1);
        assert_eq!(chunk.processing_notes[0], "Test note");
    }
}