#!/bin/sh
cd "`dirname $0`"
(cd actyx; cargo fmt -- --config imports_granularity=Crate)
(cd sdk; cargo fmt -- --config imports_granularity=Crate)
(cd release; cargo fmt -- --config imports_granularity=Crate)
