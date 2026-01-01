//! Document processing capability.

use crate::BotticelliDriver;

/// Trait for models that support document inputs (PDF, DOCX, etc.).
pub trait DocumentProcessing: BotticelliDriver {
    /// Supported document formats (MIME types).
    fn supported_document_formats(&self) -> &[&'static str] {
        &[
            "application/pdf",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "text/plain",
            "text/markdown",
        ]
    }

    /// Maximum document size in bytes.
    fn max_document_size_bytes(&self) -> usize {
        10 * 1024 * 1024 // 10MB default
    }

    /// Maximum number of pages per document.
    fn max_document_pages(&self) -> usize {
        100
    }
}
