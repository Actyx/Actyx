#!/bin/bash
RUST_LOG=debug cargo run -- --from av1 --to av2 --app-id com.example.todo-react-actyx-v1-v2 $@
