PRAGMA wal_checkpoint(TRUNCATE);
DELETE FROM conversations WHERE id = 'stale-row';

