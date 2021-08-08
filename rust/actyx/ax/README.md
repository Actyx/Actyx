## Actyx CLI
This is a command line utility script used to interact with Actyx's services.

## Example commands:
Depending of the command, you might need:
* appmgmtd && docker running
* logscvd running

Some of them should work without any of these. Please refer to the integration tests for a more complete insight.

### Logs:
cargo run --bin actyx-cli -- logs tail --entries 2 localhost

## Integration tests
`tests` folder contains some integration tests, for testing the interaction with the console service's components 