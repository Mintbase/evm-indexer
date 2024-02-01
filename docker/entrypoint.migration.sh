#!/bin/sh

# Construct DATABASE_URL from STORE_URL and NETWORK
export DATABASE_URL="${STORE_URL}?options=-c search_path%3D${NETWORK}"

# Run the wait-for-it script, then execute diesel migration
./wait-for-it.sh $DB_HOST_PORT -- diesel migration run
