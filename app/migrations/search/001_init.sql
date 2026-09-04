CREATE VIRTUAL TABLE IF NOT EXISTS t_search
USING fts5(
    content,
    type UNINDEXED,         -- activity, training note
    document_id UNINDEXED,  -- original document id
    user UNINDEXED,         -- docuement's owner id
);
