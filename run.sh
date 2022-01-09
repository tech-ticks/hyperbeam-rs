#!/bin/sh
set -e

cd hyperbeam-launcher && cargo skyline install && cd ..
sleep 1 # The upload seems to run into a race condition without the sleep for some reason
cd hyperbeam-essentials && cargo skyline install --install-path rom:/hyperbeam/modpacks/techticks.testhack/plugins/libhyperbeam_essentials.nro && cd ..
cargo skyline listen
