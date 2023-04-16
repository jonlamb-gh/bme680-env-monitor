#!/usr/bin/env bash

set -euo pipefail

arm-none-eabi-objcopy \
    -O binary \
    target/thumbv7em-none-eabihf/release/bme680-env-monitor \
    target/thumbv7em-none-eabihf/release/bme680-env-monitor.bin

# 0x08040000 == slot 1
# 0x00030800 == 194K
echo "Writing bin to slot 1"
st-flash --reset --format=binary write target/thumbv7em-none-eabihf/release/bme680-env-monitor.bin 0x08040000

exit 0
