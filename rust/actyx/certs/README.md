# Auth

## Create dev cert

Exec:

```sh
# Generate dev certificate
cargo run --bin ax-dev-cert -- create --actyx-private-key 0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w= --dev-private-key 08lUw93C+xzdxBcsYOoPVjzn8IHPJtnJW9Y/WyEu4v64= --app-domains com.actyx.* com.example.*

# If dev key is omitted, one will be generated. Actyx and dev keys could be provided in the form of env vars
export ACTYX_PRIVATE_KEY=0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=
cargo run --bin ax-dev-cert -- create --app-domains com.actyx.* com.example.*
```

Corresponding Actyx pub key `075i62XGQJuXjv6nnLQyJzECZhF29acYvYeEOJ3kc5M8=`
Corresponding Dev pub key `0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0=`

Result:

```json
{
  "devPrivkey":"08lUw93C+xzdxBcsYOoPVjzn8IHPJtnJW9Y/WyEu4v64=",
  "devPubkey":"0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0=",
  "appDomains":["com.actyx.*","com.example.*"],
  "axSignature":"8Bl3zCNno5GbpKUoati7CiFgr0KGwNHB1kTwBVKzO9pzW07hFkkQ+GXvyc9QaWhHT5aXzzO+mVrx3eiC7TREAQ=="
}
```

## Sign app manifest

```sh
# Create input files
cat > dev_cert.json << EOF
{
  "devPrivkey":"08lUw93C+xzdxBcsYOoPVjzn8IHPJtnJW9Y/WyEu4v64=",
  "devPubkey":"0nz1YdHu/JDmS6CImcZgOj9Y960sJOPrbZHAJO107qW0=",
  "appDomains":["com.actyx.*","com.example.*"],
  "axSignature":"8Bl3zCNno5GbpKUoati7CiFgr0KGwNHB1kTwBVKzO9pzW07hFkkQ+GXvyc9QaWhHT5aXzzO+mVrx3eiC7TREAQ=="
}
EOF
cat > app_manifest.json << EOF
{
  "appId": "com.actyx.auth-test",
  "displayName": "auth test app",
  "version": "v0.0.1"
}
EOF

# Sign manifest 
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
