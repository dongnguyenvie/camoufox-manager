-- 0013_proxy_rotate — CM-3.2 (Proxy change-IP link): rotate URL columns on proxies.
ALTER TABLE proxies ADD COLUMN rotate_url TEXT;
ALTER TABLE proxies ADD COLUMN rotate_on_start INTEGER NOT NULL DEFAULT 0;
