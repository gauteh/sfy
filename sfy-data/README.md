## Build and run docker image

1) Set up postgres server (e.g. through docker as below)
2) Point the config file to the database
3) Build and and run the server

```
$ docker build -t sfy-data .
$ docker run --name sfy-data --publish 3000:3000 --rm -it sfy-data
```

## Postgres

```
$ docker run --name my-postgres --env POSTGRES_PASSWORD=sfytest --publish 5432:5432 --rm -it postgres
```

## Updating the sqlx file:

1) Start postgres server as above
2) sqlx database create
3) sqlx migrate run --source migrations/postgres
3) cargo sqlx prepare

