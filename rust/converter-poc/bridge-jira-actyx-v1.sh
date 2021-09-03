#!/bin/bash
RUST_LOG=debug cargo run -- \
  --from jv1 \
  --to av1 \
  --app-id com.example.bridge-jira-v1-actyx-v1 \
  --tag-mapping '{ "Task": "task", "TaskList": "task-list" }' $@
