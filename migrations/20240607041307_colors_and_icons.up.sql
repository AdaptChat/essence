CREATE TYPE gradient_stop AS (
    position REAL, -- 0.0 to 1.0
    color INTEGER
);

CREATE TYPE gradient_type AS (
    angle REAL,
    stops gradient_stop[]
);

ALTER TABLE roles
    ALTER COLUMN gradient DROP NOT NULL,
    ALTER COLUMN gradient DROP DEFAULT,
    ALTER COLUMN gradient TYPE gradient_type USING NULL,
    ADD COLUMN IF NOT EXISTS icon TEXT;

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS color INTEGER,
    ADD COLUMN IF NOT EXISTS gradient gradient_type;