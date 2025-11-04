CREATE TABLE IF NOT EXISTS t_user_preferences (
    user_id TEXT NOT NULL,
    preference_key TEXT NOT NULL,
    preference_value TEXT NOT NULL,
    PRIMARY KEY (user_id, preference_key)
);

CREATE INDEX IF NOT EXISTS t_user_preferences_user_id_idx
ON t_user_preferences(user_id);
