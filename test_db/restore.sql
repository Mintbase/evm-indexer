\set ON_ERROR_STOP on

-- -- Drop the existing database (if it exists)
DROP DATABASE IF EXISTS postgres;

-- Create a new database
CREATE DATABASE postgres;

-- Connect to the newly created database
\c postgres;

-- Restore the database from the backup file
\i /docker-entrypoint-initdb.d/sample_events.sql;

-- Exit psql
\q
