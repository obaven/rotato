# Vaultwarden-driven secret rotation (scaffold)

This is a starter implementation to rotate app secrets via Vaultwarden and Sealed Secrets.

What it does today
- Loads `rotation/config.yaml` (copy from `config.example.yaml`).
- For each target entry, generates a high-entropy secret (>=600 bits), seals it with the live Sealed Secrets cert, commits/pushes to the target repo, and triggers an `argocd app sync`.
- Leaves TODO hooks to integrate the Vaultwarden API and to write run logs.

What still needs wiring
- Vaultwarden API write-back (store current/previous + notes).
- Persist run logs to a ConfigMap and Vaultwarden notes.
- Finalize config entries (paths/keys) per app/env.
- CronWorkflow to run weekly in-cluster.

How to run manually (off-cluster)
```bash
VW_ROTATION_CONFIG=apps/security/vaultwarden/rotation/config.yaml \
./apps/security/vaultwarden/rotation/rotate_secrets.py
```
Requires `kubeseal`, `git`, `argocd` CLIs, and repo push access.

Planned CronWorkflow
- Will run inside the cluster (Argo Workflows) weekly.
- Will read Vaultwarden API token from a SealedSecret (e.g., `vaultwarden-rotation-token` in `argocd` ns).
- Will fetch the Sealed Secrets cert each run (`kubeseal --fetch-cert`).
- Will execute the rotation script in a container with `kubeseal`, `git`, `argocd`, `python3`.
