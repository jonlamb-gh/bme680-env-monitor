#!/usr/bin/env bash

set -euo pipefail

arm-none-eabi-objcopy \
    -O binary \
    target/thumbv7em-none-eabihf/release/bme680-env-monitor \
    target/thumbv7em-none-eabihf/release/bme680-env-monitor.bin

# 0x08010000 == slot 0
# 0x00030800 == 194K
echo "Writing bin to slot 0"
st-flash --reset --format=binary --flash=194k write target/thumbv7em-none-eabihf/release/bme680-env-monitor.bin 0x08010000

exit 0
