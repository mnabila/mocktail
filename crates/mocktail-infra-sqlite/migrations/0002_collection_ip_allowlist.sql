-- Per-collection client IP/CIDR allowlist for the mock runtime.
-- Empty JSON array means allow all, so existing rows are unaffected.
ALTER TABLE collections ADD COLUMN ip_allowlist TEXT NOT NULL DEFAULT '[]';
