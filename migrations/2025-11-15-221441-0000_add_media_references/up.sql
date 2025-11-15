-- Create media_references table for storing metadata about media files
CREATE TABLE media_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_type TEXT NOT NULL CHECK (media_type IN ('image', 'audio', 'video')),
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_hash TEXT NOT NULL,
    storage_backend TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    uploaded_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMP,
    access_count INT DEFAULT 0,
    
    -- Optional metadata
    width INT,
    height INT,
    duration_seconds REAL,
    
    CONSTRAINT unique_content UNIQUE (content_hash)
);

-- Create indexes for common queries
CREATE INDEX idx_media_content_hash ON media_references(content_hash);
CREATE INDEX idx_media_type ON media_references(media_type);
CREATE INDEX idx_media_storage ON media_references(storage_backend, storage_path);

-- Add foreign key to act_inputs
ALTER TABLE act_inputs 
ADD COLUMN media_ref_id UUID REFERENCES media_references(id) ON DELETE SET NULL;

CREATE INDEX idx_act_inputs_media_ref ON act_inputs(media_ref_id);

-- Add comments for documentation
COMMENT ON TABLE media_references IS 'Metadata for media files stored outside the database';
COMMENT ON COLUMN media_references.content_hash IS 'SHA-256 hash for deduplication';
COMMENT ON COLUMN media_references.storage_backend IS 'Storage backend type: filesystem, s3, postgres, etc.';
COMMENT ON COLUMN media_references.storage_path IS 'Backend-specific path or key to the media file';
COMMENT ON COLUMN act_inputs.media_ref_id IS 'Reference to media stored in media_references table';
