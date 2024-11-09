# rekcod

docker

## rekcod cli

```
cargo run --package rekcod --bin rekcod
```

## rekcod dashboard

```
VITE_PUBLIC_PATH=rekcod npm run build
cargo build
```

## remove unused dependencies

- Install machete:

```
cargo install cargo-machete
```

- Find unused dependencies in the project:

```
cargo machete --with-metadata
```

- To fix unused dependencies:

```
cargo machete --fix
```

## rust cross build

```
cargo install cross --git https://github.com/cross-rs/cross
```

## rekcod docker

```
cargo run --package rekcod --bin rekcod -- -
-rekcod-config ./target/config/rekcod.json docker --node 192.168.31.246 info
```

## rekcod docker-compose(docker compose)
