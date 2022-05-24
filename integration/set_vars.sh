# this script must be sourced, e.g.: . set_vars.sh
export AUTH_TEST_LICENSE=`vault kv get -field=test_license secret/sec.actyx/signing/actyx`
export AUTH_TEST_NODE_LICENSE=`vault kv get -field=test_node_license secret/sec.actyx/signing/actyx`
export AUTH_TEST_SIGNATURE=`vault kv get -field=test_signature secret/sec.actyx/signing/actyx`
export ACTYX_PUBLIC_KEY=`vault kv get -field=public secret/sec.actyx/signing/actyx`
export ACTYX_VERSION=`cargo run --manifest-path ../rust/release/Cargo.toml -q get-actyx-version actyx`
export ACTYX_VERSION_CLI=`cargo run --manifest-path ../rust/release/Cargo.toml -q get-actyx-version cli`
