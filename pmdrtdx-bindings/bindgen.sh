#!/bin/sh
set -e

bindgen --no-layout-tests --no-hash "*" --no-derive-debug -o src/lib.rs src/wrapper.hpp