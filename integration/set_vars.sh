# this script must be sourced, e.g.: . set_vars.sh
export AUTH_TEST_LICENSE=`vault kv get -field=test_license secret/sec.actyx/signing/actyx`
export AUTH_TEST_SIGNATURE=`vault kv get -field=test_signature secret/sec.actyx/signing/actyx`
