set -x
set -e pipefail

#check if psql and sqlx-cli are installed first
if ! [ -x "$(command -v psql)" ]; then
	echo >&2 "Error: psql is not installed"
	exit 1
else
  echo >&2 "psql is installed"
fi

if ! [ -x "$(command -v sqlx)" ]; then
	echo >&2 "Error: sqlx is not installed"
	echo >&2 "Use:  cargo install --version='~0.6' sqlx-cli --no-default-features --features rustls, postgres"
	exit 1
else
  echo >&2 "sqlx is installed"
fi

#check if a custom user has been set, otherwise default to 'postgres'
DB_USER="${POSTGRES_USER:=postgres}"
#check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
#check if a custom database name has been set, otherwise default to 'newsletter'
DB_NAME="${POSTGRES_DB:=newsletter}"
#check if a custom port has been set, otherwise default to '5432'
DB_PORT="${POSTGRES_PORT:=5432}"
#check if a custom host has been set, otherwise default to 'localhost'
DB_HOST="${DB_HOST:=localhost}"


#launch postgres using Docker
if [[ -z ${SKIP_DOCKER} ]]
then
  echo >&2 "Starting Docker Container"
  docker run \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p 5430:5432 \
    -d postgres \
    postgres -N 1000
    #increased maximum number of connections for testing purpose
else
    echo >&2 "Docker is probably running already. check for postgress container with docker ps; SKIP_DOCKER is set to: $SKIP_DOCKER"
fi

#keep pinging postgres until it is ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
	>&2 echo "postgress is still unavailable -- sleeping"
	sleep 3
done

echo >&2 "postgres is up and running on port ${DB_PORT} - running migrations now.."

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create

#Adding a Migration
#sqlx migrate add create_subscription_table # we only need this if there is no file inside migrations folder

sqlx migrate run

echo >&2 "Postgres has been migrated. Ready to go!"

