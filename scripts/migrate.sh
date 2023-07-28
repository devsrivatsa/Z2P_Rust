sqlx database create

#Adding a Migration
sqlx migrate add create_subscription_table

sqlx migrate run

echo >&2 "Postgres has been migrated. Ready to go!"