#!/bin/bash
# update_authentik_admin.sh
# Sets the authentik admin user's password inside the running server pod.

set -e

APP_NAMESPACE="${AUTHENTIK_NAMESPACE:-authentik-prod}"
ADMIN_USERNAME="${ROTATOR_KEY_USERNAME:-admin}"
ADMIN_PASSWORD="${ROTATOR_KEY_PASSWORD:?ROTATOR_KEY_PASSWORD required}"
CONFIG_SECRET="${AUTHENTIK_CONFIG_SECRET:-authentik-config-secrets}"
REDIS_SECRET="${AUTHENTIK_REDIS_SECRET:-redis-authentik-user-credentials}"
ADMIN_EMAIL="${AUTHENTIK_ADMIN_EMAIL:-admin@example.com}"

if [ -z "$ADMIN_USERNAME" ]; then
    echo "ERROR: Missing admin username (ROTATOR_KEY_USERNAME)."
    exit 1
fi

echo "[update_authentik_admin] Locating authentik server pod in namespace '$APP_NAMESPACE'..."
SERVER_POD=$(kubectl get pod -n "$APP_NAMESPACE" -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}')

if [ -z "$SERVER_POD" ]; then
    echo "ERROR: Could not find authentik server pod."
    kubectl get pods -n "$APP_NAMESPACE" || true
    exit 1
fi

# Pull DB/Redis connection info from secrets so we can connect even before pods restart
db_host=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_POSTGRESQL__HOST}' | base64 -d)
db_name=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_POSTGRESQL__NAME}' | base64 -d)
db_user=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_POSTGRESQL__USER}' | base64 -d)
db_pass=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_POSTGRESQL__PASSWORD}' | base64 -d)
db_port=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_POSTGRESQL__PORT}' | base64 -d)
redis_host=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_REDIS__HOST}' | base64 -d)
redis_port=$(kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.AUTHENTIK_REDIS__PORT}' | base64 -d)
redis_user=$(kubectl get secret "$REDIS_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.username}' | base64 -d)
redis_pass=$(kubectl get secret "$REDIS_SECRET" -n "$APP_NAMESPACE" -o jsonpath='{.data.password}' | base64 -d)

echo "[update_authentik_admin] Setting password for user '$ADMIN_USERNAME' via Django shell..."
kubectl exec -n "$APP_NAMESPACE" "$SERVER_POD" -- env \
  DJANGO_SETTINGS_MODULE=authentik.root.settings \
  PYTHONPATH=/authentik \
  AUTHENTIK_POSTGRESQL__HOST="$db_host" \
  AUTHENTIK_POSTGRESQL__NAME="$db_name" \
  AUTHENTIK_POSTGRESQL__USER="$db_user" \
  AUTHENTIK_POSTGRESQL__PASSWORD="$db_pass" \
  AUTHENTIK_POSTGRESQL__PORT="$db_port" \
  AUTHENTIK_REDIS__HOST="$redis_host" \
  AUTHENTIK_REDIS__PORT="$redis_port" \
  AUTHENTIK_REDIS__USERNAME="$redis_user" \
  AUTHENTIK_REDIS__PASSWORD="$redis_pass" \
  AUTHENTIK_BOOTSTRAP_PASSWORD="$ADMIN_PASSWORD" \
  AUTHENTIK_BOOTSTRAP_EMAIL="$ADMIN_EMAIL" \
  ADMIN_USERNAME="$ADMIN_USERNAME" \
  python - <<'PY'
import os
import django
django.setup()
from authentik.core.models import User

username = os.environ.get("ADMIN_USERNAME", "admin")
password = os.environ["AUTHENTIK_BOOTSTRAP_PASSWORD"]
email = os.environ.get("AUTHENTIK_BOOTSTRAP_EMAIL", "admin@example.com")

user, _ = User.objects.get_or_create(username=username, defaults={"email": email})
user.set_password(password)
user.is_superuser = True
user.is_staff = True
user.save()
print(f"[update_authentik_admin] Password set for {username}.")
PY

echo "[update_authentik_admin] Done."
