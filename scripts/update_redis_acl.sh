#!/bin/bash
# update_redis_acl.sh
# Updates a Redis user's password via ACL command.

set -e

# Validate Inputs
if [ -z "$REDIS_SERVICE" ] || [ -z "$REDIS_NAMESPACE" ] || [ -z "$REDIS_ADMIN_SECRET" ]; then
    echo "ERROR: Missing required env vars: REDIS_SERVICE, REDIS_NAMESPACE, REDIS_ADMIN_SECRET"
    exit 1
fi

if [ -z "$ROTATOR_KEY_USERNAME" ] || [ -z "$ROTATOR_KEY_PASSWORD" ]; then
    echo "ERROR: Missing ROTATOR_KEY_* env vars (not running via rotato?)"
    exit 1
fi

USERNAME="$ROTATOR_KEY_USERNAME"
# The new password to set
NEW_PASSWORD="$ROTATOR_KEY_PASSWORD"

echo "[update_redis_acl] Fetching Redis Admin Password from Secret '$REDIS_ADMIN_SECRET'..."
# Fetch the 'password' key from the admin secret
ADMIN_PASSWORD=$(kubectl get secret "$REDIS_ADMIN_SECRET" -n "$REDIS_NAMESPACE" -o jsonpath='{.data.password}' | base64 -d)

if [ -z "$ADMIN_PASSWORD" ]; then
    echo "ERROR: Could not retrieve admin password."
    exit 1
fi

echo "[update_redis_acl] Updating ACL for user '$USERNAME'..."

# Construct Redis CLI command
# Usage: redis-cli -h HOST -a ADMIN_PASS ACL SETUSER USERNAME on >NEW_PASS
# We user "on" to enable, ">Pass" to add password.
# Note: This appends a password. We might want to reset it? 
# "resetpass" clears existing, ">..." adds new.
# Full: ACL SETUSER <user> resetpass ><newpass> on

# Use a temporary pod? Or exec into existing?
# Exec into existing is easier if we can find one.
# Or just run a throwaway pod with redis-cli.
# Assuming we have network access from where rotator runs (it runs locally or in CI runner).
# If local, we can port-forward? 
# IF running in cluster (Argo Workflows), we can access service directly.
# The user prompt implies running locally for now ("gitops" folder), but design should be robust.
# Let's try port-forward if we can't connect? No, let's assume `redis-cli` is installed locally or we use `kubectl exec`.

# Let's use `kubectl exec` into the redis primary for maximum reliability (no external auth exposures).
# Warning: This assumes a specific pod naming convention or label.
# Correct selector based on observed labels:
POD_SELECTOR="app.kubernetes.io/name=redis,app.kubernetes.io/component=master"
# Fallback to just "app=redis" if needed.

# Find a running redis pod
REDIS_POD=$(kubectl get pod -n "$REDIS_NAMESPACE" -l "$POD_SELECTOR" -o jsonpath='{.items[0].metadata.name}')

if [ -z "$REDIS_POD" ]; then
    echo "WARNING: specific selector failed, trying generic app=redis-prod"
     REDIS_POD=$(kubectl get pod -n "$REDIS_NAMESPACE" -l "app.kubernetes.io/instance=redis-prod" -o jsonpath='{.items[0].metadata.name}')
fi

if [ -z "$REDIS_POD" ]; then
    echo "ERROR: Could not find Redis pod to exec into."
    exit 1
fi

echo "[update_redis_acl] Executing on Pod '$REDIS_POD'..."

kubectl exec -n "$REDIS_NAMESPACE" "$REDIS_POD" -- \
    redis-cli -a "$ADMIN_PASSWORD" \
    ACL SETUSER "$USERNAME" resetpass ">$NEW_PASSWORD" on "~*" "&*" "+@all"

echo "[update_redis_acl] Success."
