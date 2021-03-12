# Logsvcd

Implements the logging functionality of the ActyxOS Console Service

## Testing locally

2. Run logscvd:
```
cargo run --bin logsvcd -- --source-id sourceid --serial-number serial -vvvv 
```

3. Open a tail connection:
```
curl -vvv http://localhost:4458/logs/stream
```

4. Sample json file:
```
{ "severity": "info", "message": "Your message", "logName": "yourLogName", "producerName": "com.mycompany.myapp", "producerVersion": "1.0.3", "additionalData": { "yourKey": "Your value" }, "labels": { "com.app-builder-ltd.gdpr-relevant": "false" } }
``` 
5. Send it with:
```
curl -vvv -H "Content-type: application/json" -d @example.log  http://localhost:4458/logs
```
and check your terminal with the tail connection!

Or test it with the websocket api:
```
wscat --connect http://localhost:4457/api/v1/logs
```
Start sending logs, one per line.
