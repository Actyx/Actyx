#!/bin/bash
RUST_LOG=debug cargo run -- \
  --from av1 \
  --to jv1 \
  --app-id com.example.bridge-actyx-v1-jira-v1 \
  --tag-mapping '{ "task": "Task", "task-list": "TaskList" }' \
  --query "FROM 'task-list'" \
  --query "FROM allEvents" \
  $@
