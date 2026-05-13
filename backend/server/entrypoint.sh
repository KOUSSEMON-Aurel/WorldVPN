#!/bin/sh
set -e

echo "🔄 Running database migrations with dbmate..."
dbmate --url "$DATABASE_URL" --migrations-dir /usr/local/bin/migrations --no-dump-schema up

echo "✅ Migrations applied. Starting WorldVPN server..."
exec /usr/local/bin/vpn-server
