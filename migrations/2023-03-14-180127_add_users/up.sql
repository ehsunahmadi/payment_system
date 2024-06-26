CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    balance TEXT NOT NULL DEFAULT '0.00'

);

CREATE TABLE payments (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    amount TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    session_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);