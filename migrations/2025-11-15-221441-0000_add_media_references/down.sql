-- Remove the media_ref_id column from act_inputs
DROP INDEX IF EXISTS idx_act_inputs_media_ref;
ALTER TABLE act_inputs DROP COLUMN IF EXISTS media_ref_id;

-- Drop the media_references table
DROP INDEX IF EXISTS idx_media_storage;
DROP INDEX IF EXISTS idx_media_type;
DROP INDEX IF EXISTS idx_media_content_hash;

DROP TABLE IF EXISTS media_references;
