#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    SELECT 'CREATE DATABASE "hive-local" WITH OWNER "$POSTGRES_USER"'
    WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'hive-local')\gexec
    SELECT 'CREATE DATABASE "hive-test" WITH OWNER "$POSTGRES_USER"'
    WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'hive-test')\gexec
    GRANT ALL PRIVILEGES ON DATABASE "hive-local" TO "$POSTGRES_USER";
    GRANT ALL PRIVILEGES ON DATABASE "hive-test" TO "$POSTGRES_USER";
EOSQL
