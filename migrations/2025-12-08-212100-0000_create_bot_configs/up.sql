-- Create bot_configs table for managing automated bots
CREATE TABLE bot_configs (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    bot_type VARCHAR(50) NOT NULL,
    narrative_id INTEGER REFERENCES narrative_executions(id) ON DELETE SET NULL,
    schedule_config JSONB,
    is_active BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_bot_configs_active ON bot_configs(is_active) WHERE is_active = true;
CREATE INDEX idx_bot_configs_type ON bot_configs(bot_type);
