-- Add migration script here
CREATE TABLE IF NOT EXISTS buoys (dev TEXT, name TEXT, PRIMARY KEY (dev));

CREATE TABLE IF NOT EXISTS events (dev TEXT, event TEXT, data BLOB, PRIMARY KEY (dev, event));
