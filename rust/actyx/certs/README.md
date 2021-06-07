# Auth

## Create dev cert

Exec:

```sh
# 
export ACTYX_PRIVATE_KEY=0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w= # Corresponding pub key `075i62XGQJuXjv6nnLQyJzECZhF29acYvYeEOJ3kc5M8=`
# Corresponding dev private key `08lUw93C+xzdxBcsYOoPVjzn8IHPJtnJW9Y/WyEu4v64=` for the passed in public key below.
cargo run --bin ax-dev-cert -- create --dev-public-key 0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0= --app-domains com.actyx.* com.example.*
```

Result:

```json
{
  "devPubkey":"0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0=",
  "appDomains":["com.actyx.*","com.example.*"],
  "axSignature":"8Bl3zCNno5GbpKUoati7CiFgr0KGwNHB1kTwBVKzO9pzW07hFkkQ+GXvyc9QaWhHT5aXzzO+mVrx3eiC7TREAQ=="
}
```

## Sign app manifest

Create dev_cert.json:

```json
{
  "devPubkey":"0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0=",
  "appDomains":["com.actyx.*","com.example.*"],
  "axSignature":"8Bl3zCNno5GbpKUoati7CiFgr0KGwNHB1kTwBVKzO9pzW07hFkkQ+GXvyc9QaWhHT5aXzzO+mVrx3eiC7TREAQ=="
}
```

Create app_manifest.json:

```json
{
  "appId": "com.actyx.auth-test",
  "displayName": "auth test app",
  "version": "v0.0.1"
}
```

Sign manifest

```sh
# Make sure your `ax users keygen` key pair matches the one used for dev cert generation or pass in a custom identity path
cargo run --bin ax -- apps sign dev_cert.json app_manifest.json
```

Result:

```json
{
  "appId": "com.actyx.auth-test",
  "displayName": "auth test app",
  "version": "v0.0.1",
  "signature": "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w=="
}
```

## Get token

Start node:

```bash
# Set proper actyx public key
export AX_PUBLIC_KEY=075i62XGQJuXjv6nnLQyJzECZhF29acYvYeEOJ3kc5M8=
cargo run --bin actyx-linux
```

```bash
curl \
    -s -X "POST" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -d '{"appId": "com.actyx.auth-test", "displayName": "auth test app","version": "v0.0.1", "signature": "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w=="}' \
    http://localhost:4454/api/v2/auth
```
