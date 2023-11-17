#!/bin/sh
cd "`dirname $0`"
(cd actyx; cargo +nightly fmt -- --config imports_granularity=Crate)
(cd sdk; cargo +nightly fmt -- --config imports_granularity=Crate)
(cd release; cargo +nightly fmt -- --config imports_granularity=Crate)
