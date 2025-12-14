#!/bin/bash
# update_authentik_redis_config.sh
# Patch authentik's config secret with the rotated Redis username/password and re-seal it.

set -euo pipefail

APP_NAMESPACE="${AUTHENTIK_NAMESPACE:-authentik-prod}"
CONFIG_SECRET="${AUTHENTIK_CONFIG_SECRET:-authentik-config-secrets}"
SEALED_OUTPUT_PATH="${AUTHENTIK_CONFIG_SEALED_PATH:-apps/security/authentik/overlays/prod/components/sealed_secrets/sealed-authentik-config-secrets.yaml}"
CERT_PATH="${SEALED_SECRETS_CERT:-apps/security/sealed-secrets/secrets/sealed-secrets-public-key.crt}"

if [ -z "${ROTATOR_KEY_USERNAME:-}" ] || [ -z "${ROTATOR_KEY_PASSWORD:-}" ]; then
    echo "ERROR: ROTATOR_KEY_USERNAME/PASSWORD not set."
    exit 1
fi

echo "[update_authentik_redis_config] Patching $CONFIG_SECRET in ns=$APP_NAMESPACE..."
kubectl patch secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" --type=json -p="[
  {\"op\":\"replace\",\"path\":\"/data/AUTHENTIK_REDIS__USERNAME\",\"value\":\"$(printf '%s' \"$ROTATOR_KEY_USERNAME\" | base64 -w0)\"},
  {\"op\":\"replace\",\"path\":\"/data/AUTHENTIK_REDIS__PASSWORD\",\"value\":\"$(printf '%s' \"$ROTATOR_KEY_PASSWORD\" | base64 -w0)\"}
]"

echo "[update_authentik_redis_config] Re-sealing to $SEALED_OUTPUT_PATH..."
# Ensure output directory exists
mkdir -p "$(dirname "$SEALED_OUTPUT_PATH")"

kubectl get secret "$CONFIG_SECRET" -n "$APP_NAMESPACE" -o yaml \
  | kubeseal --format yaml --cert "$CERT_PATH" \
  > "$SEALED_OUTPUT_PATH"

echo "[update_authentik_redis_config] Done."
