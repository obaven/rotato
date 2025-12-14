# Vaultwarden Rotator Runbook

This runbook documents the Standard Operating Procedures (SOPs) for managing the Decentralized Secret Rotator.

## 🧑‍💻 Flow 1: Onboarding a New Application

**Goal**: Enable secret rotation for a new application.

1.  **Navigate to the app directory**:
    ```bash
    cd apps/my-domain/my-app
    ```
2.  **Scaffold the Configuration**:
    Run the scaffolding tool (from the `rotator_helper` directory or using the binary):
    ```bash
    # Assuming valid credentials in env
    cargo make scaffold -- --app my-app --env prod
    ```
3.  **Customize `rotation.yaml`**:
    Edit the generated file. Ensure:
    *   `vaultwarden.cipherId` matches the item in use (or use `create-items` to make one).
    *   `kubernetes.path` points to where the SealedSecret should reside.
    *   `keys` list all fields you want controlled (random or static).
4.  **Commit**:
    ```bash
    git add rotation.yaml
    git commit -m "feat: enable secret rotation for my-app"
    git push
    ```

## 🔐 Flow 2: Manual Rotation (Operator)

**Goal**: Force a rotation immediately (e.g., after a compromise or for testing).

1.  **Dry Run (Verify)**:
    Check what *would* happen without making changes.
    ```bash
    cargo make rotate-dry
    ```
    *   *Check Output*: Ensure it found your config and plans to update the correct files.
2.  **Execute Rotation**:
    ```bash
    cargo make rotate
    ```
    *   *Action*: This will update Vaultwarden, generate new SealedSecrets, and commit to Git.
3.  **Sync ArgoCD**:
    Navigate to the ArgoCD UI and Sync the application (or wait for auto-sync) to apply the new SealedSecret.

## 🤖 Flow 3: Managing the Automation (Service Worker)

**Goal**: Interact with the Kubernetes CronJob.

### Trigger a Manual Job
If you don't want to wait for the nightly schedule:
```bash
kubectl create job --from=cronjob/vaultwarden-rotator manual-rotation-01 -n vaultwarden-prod
```

### View Logs
Check the output of the rotation job:
```bash
# Get the pod name
kubectl get pods -n vaultwarden-prod -l job-name=manual-rotation-01

# Stream logs
kubectl logs -f <pod-name> -n vaultwarden-prod
```

## 🚑 Flow 4: Troubleshooting

### Incident: "Secrets are not rotating"
1.  **Check Discovery**:
    Run `cargo make rotate-dry` locally. Does it list your `rotation.yaml`?
    *   *Fix*: Ensure `rotation.yaml` is in a subdirectory of `apps/` and the "discovery" logic can find it.
2.  **Check Job Logs**:
    See "View Logs" above. Look for auth errors or "Git push rejected".

### Incident: "ArgoCD is Out of Sync"
1.  **Check Git History**:
    Did the rotator commit a change that ArgoCD is rejecting?
2.  **Check SealedSecret Validity**:
    Ensure the `kubeseal` certificate used by the rotator matches the controller in the cluster.
    *   *Fix*: Re-fetch the public cert: `kubeseal --fetch-cert ...`

## 📦 Reference: Key Types

| Key Type | usage | Example |
| :--- | :--- | :--- |
| **Random** | Passwords, Tokens | `type: random, length: 32` |
| **Static** | Usernames, Hosts | `type: static, value: "admin"` |
| **File** | Helm Values ref | `type: file, path: "values.yaml", key: "db.user"` |

### Advanced Debugging

To investigate complex issues, use granular debug flags:

| Flag | Description | Noise Level |
| :--- | :--- | :--- |
| `--debug` | Enables high-level logic logs (collection matching, policy checks). | Low |
| `--debug-api` | **Logs full JSON payloads** for API requests (`create`, `update`). Useful for checking field structure. | Medium |
| `--debug-auth` | Logs authentication token details and configuration. | Low |
| `--debug-crypto` | **Trace Logs**. Logs every MAC verification and RSA padding attempt. | High |

**Example**:
```bash
# Check why an update failed without drowning in crypto traces
cargo run -- rotate --scan --debug-api
```
