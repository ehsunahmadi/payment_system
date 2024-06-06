CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE
);

CREATE TABLE payments (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    amount REAL NOT NULL,
    status TEXT NOT NULL,
    stripe_payment_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE balances (
    user_id INTEGER NOT NULL PRIMARY KEY REFERENCES users(id),
    balance REAL NOT NULL DEFAULT 0.0
);
