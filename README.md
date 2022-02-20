# raphdb

Welcome to raphdb!

Raphdb is on disk database written in rust. It is built as an exercise to experiment/learn more aboud database designs.
Raphdb is designed in a way that allows to choose from a multitude of backend implementations (for example simple-store, LSM tree, B-tree) and implement any engine on top such as SQL, Graph, Document etc.

## Current State

Raphdb is still very much in development state and currently on contains a simple-store backend implementations and a redis-like tcp API.

## Short-term Roadmap

- Created unit test and integration test suite.
- Implement LSM backend.
- Implement B-Tree backend.
- Clean up README and create examples.

## How To

```bash
cargo run --help
```

Start a server:

```bash
cargo run start-server -b  simple-store
```

Connect to server with client:

```bash
cargo run start-client get --key key1 --value HelloWorld
cargo run start-client get --key key1
```
