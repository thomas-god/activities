DROP TABLE IF EXISTS t_search;

-- to trigger import of documents history
DELETE FROM t_metadata WHERE key = "last_import_date";

CREATE VIRTUAL TABLE IF NOT EXISTS t_search
USING fts5(
    content,
    type UNINDEXED,         -- activity, training note
    document_id UNINDEXED,  -- original document id
    user UNINDEXED,         -- docuement's owner id
    tokenize="trigram"
);
