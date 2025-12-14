#!/bin/bash
# scripts/test_authentik_redis_stability.sh
# Verifies Authentik's connectivity to Redis and optionally rotates secrets to test resilience.

APP_NS="authentik-prod"
REDIS_SECRET="redis-authentik-user-credentials"
LOG_FILE="/tmp/authentik_stability_test.log"

log() {
    echo "[$(date)] $1" | tee -a "$LOG_FILE"
}

check_authentik_health() {
    log "Checking Authentik Health..."
    # 1. Check Pod Status
    PODS=$(kubectl get pods -n "$APP_NS" -l app.kubernetes.io/name=authentik -o jsonpath='{.items[*].status.phase}')
    if [[ "$PODS" != *"Running"* ]]; then
        log "ERROR: Authentik pods are not running: $PODS"
        return 1
    fi

    # 2. Check Logs for Redis Errors (simple grep)
    log "Checking logs for Redis connection errors..."
    ERRORS=$(kubectl logs -n "$APP_NS" -l app.kubernetes.io/name=authentik --tail=100 | grep -i "redis" | grep -iE "refused|error|denied|auth")
    if [ ! -z "$ERRORS" ]; then
        log "WARNING: Found potential Redis errors in recent logs:"
        echo "$ERRORS" | tee -a "$LOG_FILE"
        # We don't fail immediately, but warn
    else
        log "No recent Redis errors found in logs."
    fi
    
    log "Authentik appears healthy."
    return 0
}

rotate_and_verify() {
    log "Starting Rotation Stability Test..."
    
    # Pre-check
    check_authentik_health || exit 1
    
    log "Rotating Redis Credential using Rotator..."
    # Run rotation for the specific config that contains our secret.
    # Note: This rotates ALL secrets in that config.
    cd /home/jdean/gitops/apps/security/vaultwarden/rotation/rotator_helper
    cargo run -- rotate --config /home/jdean/gitops/apps/security/authentik/rotation.yaml
    cd -
    # For this test script, we assume the hook is configured or we do it manually.
    
    log "Restarting Authentik Server (simulating hook action)..."
    kubectl rollout restart deployment/authentik-server -n "$APP_NS"
    kubectl rollout status deployment/authentik-server -n "$APP_NS"
    
    log "Waiting for stabilization..."
    sleep 10
    
    check_authentik_health
}

# Main
case "$1" in
    "check")
        check_authentik_health
        ;;
    "rotate-test")
        rotate_and_verify
        ;;
    *)
        echo "Usage: $0 {check|rotate-test}"
        exit 1
        ;;
esac
