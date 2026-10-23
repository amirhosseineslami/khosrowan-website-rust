-- Add an English price label to service translations (e.g. "on request").
ALTER TABLE service_translations ADD COLUMN price_from TEXT;
