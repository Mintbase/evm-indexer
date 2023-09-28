FROM postgres:latest

# Copy backup file and unzip into the container
COPY sample_events.sql.gz /docker-entrypoint-initdb.d/
RUN gunzip /docker-entrypoint-initdb.d/sample_events.sql.gz

COPY restore.sql /docker-entrypoint-initdb.d/

# Set the environment variables for PostgreSQL
ENV POSTGRES_USER postgres
ENV POSTGRES_PASSWORD postgres
ENV POSTGRES_DB arak
