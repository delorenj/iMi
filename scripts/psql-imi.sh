#!/usr/bin/env bash
# PostgreSQL connection helper for iMi database
# Usage: ./scripts/psql-imi.sh [psql arguments]

set -euo pipefail

# Connection parameters
export PGHOST=192.168.1.12
export PGPORT=5432
export PGDATABASE=imi
export PGUSER=imi
export PGPASSWORD='imi_dev_password_2026'

# Connect to iMi database
psql "$@"
