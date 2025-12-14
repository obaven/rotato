#!/bin/bash
# validate_authentik_admin.sh
# Verifies the authentik admin username/password by checking it in Django.

set -euo pipefail

APP_NAMESPACE="${AUTHENTIK_NAMESPACE:-authentik-prod}"
ADMIN_USERNAME="${ROTATOR_KEY_USERNAME:-admin}"
ADMIN_PASSWORD="${ROTATOR_KEY_PASSWORD:?ROTATOR_KEY_PASSWORD required for validation}"

echo "[validate_authentik_admin] Locating authentik server pod in namespace '$APP_NAMESPACE'..."
SERVER_POD=$(kubectl get pod -n "$APP_NAMESPACE" -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}')

if [ -z "$SERVER_POD" ]; then
    echo "ERROR: Could not find authentik server pod to validate credentials."
    kubectl get pods -n "$APP_NAMESPACE" || true
    exit 1
fi

echo "[validate_authentik_admin] Validating credentials for user '$ADMIN_USERNAME'..."
kubectl exec -n "$APP_NAMESPACE" "$SERVER_POD" -- env \
  DJANGO_SETTINGS_MODULE=authentik.root.settings \
  PYTHONPATH=/authentik \
  ADMIN_USERNAME="$ADMIN_USERNAME" \
  ADMIN_PASSWORD="$ADMIN_PASSWORD" \
  python - <<'PY'
import os
import django
django.setup()
from authentik.core.models import User

username = os.environ["ADMIN_USERNAME"]
password = os.environ["ADMIN_PASSWORD"]

try:
    user = User.objects.get(username=username)
except User.DoesNotExist:
    raise SystemExit(f"[validate_authentik_admin] FAIL: user '{username}' not found.")

if not user.check_password(password):
    raise SystemExit(f"[validate_authentik_admin] FAIL: password check failed for '{username}'.")

print(f"[validate_authentik_admin] OK: password valid for '{username}'.")
PY

echo "[validate_authentik_admin] Validation succeeded."
