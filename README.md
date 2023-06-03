# RuTTY - Rust TTY Server

[![Crates.io](https://img.shields.io/crates/v/rutty)](https://crates.io/crates/rutty)
[![Build](https://github.com/papigers/rutty/actions/workflows/build.yml/badge.svg)](https://github.com/papigers/rutty/actions/workflows/build.yml)

<img src="web/assets/terminal.png" width="100px" >

RuTTY (aka Ruthie) is a CLI-powered websocket server written in Rust that allows you to expose your commands via browser.
RuTTY was written with the sole-purpose of me wanting to expermient with Rust.

## How it works?

RuTTY was **heavily** inspired by a very similar tool written in Go, called [GoTTY](https://github.com/yudai/gotty).

RuTTY run a command for each client connection, forwards the TTY stdout to the client, and forwards the client input to the TTY stdin. RuTTY uses [xterm.js](https://github.com/xtermjs/xterm.js) to show a TTY display on the webpage.

## Installtion

`cargo install rutty`

## Usage

To run rutty simply run `rutty` and add your command and any optional arguments to that command, e.g. `rutty vi test.txt`.

### Options

| Option            | Description                              | Default                            |
| ----------------- | ---------------------------------------- | ---------------------------------- |
| -address (-a)     | Server listening IP address              | `0.0.0.0` (All interfaces)         |
| -port (-p)        | Server listening port                    | `3000`                             |
| -allow-write (-w) | Wether clients are allowed to pass input | `false`                            |
| --title (-t)      | HTML page title                          | `RuTTY Server`                     |
| --reconnect (-r)  | Automatic reconnection delay             | `None` (no automatic reconnection) |

## Development

Clone the repository, install `rust`, `node` & `yarn`.

### Build

Debug build: `make build`.\
Release build: `make build release=1`

### Run

`cargo run -- <COMMAND> <ARGS>`

### Web development

An automatic hot-reload of the static files can be done by running `yarn start` inside the `web` directory, which will use `parcel` to start a development server on http://localhost:1234 that is proxied to your RuTTY server running on port 3000.

## License

MIT
