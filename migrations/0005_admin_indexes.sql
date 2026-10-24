-- Indexes for admin list queries that filter by status.
CREATE INDEX IF NOT EXISTS idx_messages_status ON messages(status);
CREATE INDEX IF NOT EXISTS idx_subscribers_status ON subscribers(status);
