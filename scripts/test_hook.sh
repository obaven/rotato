#!/bin/bash
# test_hook.sh
# Simple script to verify that rotator_helper hooks are executing correctly.

echo "[$(date)] Test Hook Executed"
echo "Args: $@"
echo "Env: SOME_VAR=${SOME_VAR}"

# Write to a tmp file for verification if needed
echo "Executed with args: $@" >> /tmp/rotator_hook_test.log
