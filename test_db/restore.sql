\set ON_ERROR_STOP on

-- -- Drop the existing database (if it exists)
DROP DATABASE IF EXISTS arak;
DROP DATABASE IF EXISTS store;

-- Create a new database
CREATE DATABASE arak;
CREATE DATABASE store;

-- Connect to the newly created database
\c arak;

-- Restore the event database from the backup file
\i /sample_events.sql;

-- Exit psql
\q
