/// A text chunk with its position metadata.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub index: usize,
}

/// Split text into chunks with paragraph-aware boundaries.
///
/// Tries to break at paragraph boundaries (double newlines). Falls back to
/// sentence boundaries, then hard character splits.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    if text.len() <= chunk_size {
        return vec![Chunk {
            text: text.to_string(),
            index: 0,
        }];
    }

    let paragraphs = split_paragraphs(text);
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut index = 0;

    for para in &paragraphs {
        if current.len() + para.len() + 2 > chunk_size && !current.is_empty() {
            chunks.push(Chunk {
                text: current.trim().to_string(),
                index,
            });
            index += 1;
            // Keep overlap from end of current chunk
            current = get_overlap_text(&current, overlap);
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);
    }

    if !current.trim().is_empty() {
        chunks.push(Chunk {
            text: current.trim().to_string(),
            index,
        });
    }

    // Handle case where a single paragraph exceeds chunk_size
    let mut result = Vec::new();
    for chunk in chunks {
        if chunk.text.len() <= chunk_size {
            result.push(chunk);
        } else {
            // Hard split on sentence or character boundaries
            let sub_chunks = hard_split(&chunk.text, chunk_size, overlap);
            for (i, text) in sub_chunks.into_iter().enumerate() {
                result.push(Chunk {
                    text,
                    index: chunk.index + i,
                });
            }
        }
    }

    // Re-index
    for (i, chunk) in result.iter_mut().enumerate() {
        chunk.index = i;
    }

    result
}

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn get_overlap_text(text: &str, overlap: usize) -> String {
    if overlap == 0 || text.len() <= overlap {
        return String::new();
    }
    let start = text.len().saturating_sub(overlap);
    // Try to break at a word boundary
    let slice = &text[start..];
    if let Some(pos) = slice.find(' ') {
        slice[pos..].trim_start().to_string()
    } else {
        slice.to_string()
    }
}

fn hard_split(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = (start + chunk_size).min(text.len());
        // Try to break at sentence boundary
        let slice = &text[start..end];
        let actual_end = if end < text.len() {
            if let Some(pos) = slice.rfind(". ") {
                start + pos + 2
            } else if let Some(pos) = slice.rfind('\n') {
                start + pos + 1
            } else {
                end
            }
        } else {
            end
        };

        chunks.push(text[start..actual_end].trim().to_string());
        start = actual_end.saturating_sub(overlap);
    }

    chunks.retain(|c| !c.is_empty());
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text_single_chunk() {
        let chunks = chunk_text("Hello world", 100, 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
    }

    #[test]
    fn test_paragraph_splitting() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_text(text, 30, 0);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].text.contains("First"));
    }

    #[test]
    fn test_overlap() {
        let text = "A long first paragraph with lots of words and content here.\n\nSecond paragraph also has content.";
        let chunks = chunk_text(text, 60, 20);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let text = "Para one.\n\nPara two.\n\nPara three.\n\nPara four.";
        let chunks = chunk_text(text, 20, 0);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }
}
