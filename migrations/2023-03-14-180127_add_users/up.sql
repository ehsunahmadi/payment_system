CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE
);

CREATE TABLE payments (
    id INTEGER NOT NULL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    amount INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    stripe_payment_id TEXT NOT NULL DEFAULT 'Yochangeme',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE balances (
    user_id INTEGER NOT NULL PRIMARY KEY REFERENCES users(id),
    balance INTEGER NOT NULL DEFAULT 0
);
