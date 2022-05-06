CREATE TABLE buoys (dev TEXT, name TEXT, PRIMARY KEY (dev));

CREATE TABLE events (dev TEXT, event TEXT, data BLOB, PRIMARY KEY (dev, event));
