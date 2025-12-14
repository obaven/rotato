#!/bin/bash
# update_postgres_user.sh
# Updates a Postgres user's password via psql.

set -e

# Validate Inputs
if [ -z "$PG_NAMESPACE" ] || [ -z "$PG_ADMIN_SECRET" ]; then
    echo "ERROR: Missing required env vars: PG_NAMESPACE, PG_ADMIN_SECRET"
    exit 1
fi

if [ -z "$ROTATOR_KEY_USERNAME" ] || [ -z "$ROTATOR_KEY_PASSWORD" ]; then
    echo "ERROR: Missing ROTATOR_KEY_* env vars (not running via rotato?)"
    exit 1
fi

USERNAME="$ROTATOR_KEY_USERNAME"
NEW_PASSWORD="$ROTATOR_KEY_PASSWORD"
PG_DATABASE="${PG_DATABASE:-postgres}"
PG_HOST="${PG_HOST:-127.0.0.1}"
PG_PORT="${PG_PORT:-5432}"
APP_SECRET_NAME="${PG_APP_SECRET:-}"
APP_USER_KEY="${PG_APP_USER_KEY:-AUTHENTIK_POSTGRESQL__USER}"
APP_PASSWORD_KEY="${PG_APP_PASSWORD_KEY:-AUTHENTIK_POSTGRESQL__PASSWORD}"
APP_SECRET_NAMESPACE="${PG_APP_NAMESPACE:-$PG_NAMESPACE}"

echo "[update_postgres_user] Fetching Postgres Admin Password from Secret '$PG_ADMIN_SECRET'..."
# Key for password often 'password' or 'postgres-password'
ADMIN_PASSWORD=$(kubectl get secret "$PG_ADMIN_SECRET" -n "$PG_NAMESPACE" -o jsonpath='{.data.password}' | base64 -d)

# Find Postgres Pod
# Common selectors; try multiple and pick the first running pod
POD_SELECTORS=(
  "cnpg.io/cluster=postgresql,cnpg.io/instanceRole=primary"
  "role=primary"
  "app.kubernetes.io/name=postgresql,app.kubernetes.io/instance=postgresql-prod-primary"
  "app.kubernetes.io/name=postgresql"
  "app=postgresql"
)

PG_POD=""
for selector in "${POD_SELECTORS[@]}"; do
  PG_POD=$(kubectl get pod -n "$PG_NAMESPACE" -l "$selector" --field-selector=status.phase=Running -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
  if [ -n "$PG_POD" ]; then
    break
  fi
done

# Fallback: pick any running pod
if [ -z "$PG_POD" ]; then
  PG_POD=$(kubectl get pod -n "$PG_NAMESPACE" --field-selector=status.phase=Running -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
fi

if [ -z "$PG_POD" ]; then
    echo "ERROR: Could not find Postgres pod to exec into in namespace '$PG_NAMESPACE'."
    kubectl get pods -n "$PG_NAMESPACE" || true
    exit 1
fi

echo "[update_postgres_user] Executing on Pod '$PG_POD'..."

# Command: ALTER USER "username" WITH PASSWORD 'password';
# We use PGPASSWORD env var for auth.
# Note: "postgres" is usually the superuser.
# We execute psql inside the pod.
kubectl exec -n "$PG_NAMESPACE" "$PG_POD" -- env PGPASSWORD="$ADMIN_PASSWORD" psql -U postgres -c "ALTER USER \"$USERNAME\" WITH PASSWORD '$NEW_PASSWORD';"

# Sanity check: authenticate with the new password
echo "[update_postgres_user] Verifying new credentials..."
kubectl exec -n "$PG_NAMESPACE" "$PG_POD" -- env PGPASSWORD="$NEW_PASSWORD" psql -h "$PG_HOST" -p "$PG_PORT" -U "$USERNAME" -d "$PG_DATABASE" -c "SELECT 1" >/dev/null
echo "[update_postgres_user] Verification succeeded for user '$USERNAME' on database '$PG_DATABASE'."

# Optional: update application config secret to keep app env in sync
if [ -n "$APP_SECRET_NAME" ]; then
    echo "[update_postgres_user] Patching app secret '$APP_SECRET_NAME' (ns=$APP_SECRET_NAMESPACE) with new credentials..."
    kubectl patch secret "$APP_SECRET_NAME" -n "$APP_SECRET_NAMESPACE" -p "$(cat <<EOF
{
  "stringData": {
    "$APP_USER_KEY": "$USERNAME",
    "$APP_PASSWORD_KEY": "$NEW_PASSWORD"
  }
}
EOF
)" >/dev/null
    echo "[update_postgres_user] App secret patched."
fi

echo "[update_postgres_user] Success."
