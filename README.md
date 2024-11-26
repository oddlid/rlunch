# rlunch

A new take on [go2lunch](https://github.com/oddlid/go2lunch),
this time in Rust.

The overall purpose is to serve a website with restaurant menus,
condensed into a single site.\
Scrapers can be implemented for automatic updates of menus.\
Scrapers can be implemented in Rust and compiled in and handled by
the produced binary, like the one included for lindholmen.se, or
they could be implemented in any other language and update the DB
directly, from outside the binary.\
The DB can also be updated manually for sites and restaurants not
covered by scrapers. Just make sure to not have any overlap, as
scrapers could overwrite manually added content.

This project will produce a single binary for use in multiple sub-modes (git style):

- Run registered scrapers, either one-off, or via cron scheduling, to update the DB
- Run a web server with HTML output
- Run a web server with a REST API JSON output
- Run an "admin" web server for receiving updates to pass on to the DB (unimplemented!)

My deployed production setup runs one Docker container instance (from the
same image) for each of these modes, with the same DB container as
the backend for all.

**NB**: You currently need to build the Docker image yourself, if
running on another architecture than linux/arm64/v8, since I have
not set up automated multiplatform build (yet).

## Requirements

- Rust toolchain
- Docker
- sqlx
- Postgres (in Docker)

### Setup

- Clone the repo.
- Edit files `.env` and `docker-compose.yml` if you want to change
  settings for Postgres
- Run `docker compose up -d db`
- Run `sqlx database create`
- Run `sqlx migrate run`
- Run any subcommand you'd like to test out, via:\
  `cargo run -q --bin rlunch -- [global-options] <subcommand> [options]`\
  See `cli.rs` for options, alternatively run:\
  `cargo run -q --bin rlunch -- --help`
