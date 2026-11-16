-- 0002_proxies — phase 2 (Proxy library): look-ups by endpoint (duplicate detection on import).
CREATE INDEX IF NOT EXISTS proxies_endpoint ON proxies(host, port);
