#!/bin/bash
RUST_LOG=debug cargo run -- --from av2 --to av1 --app-id com.example.todo-react-actyx-v2-v1 $@
