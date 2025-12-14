# Vaultwarden Rotato Architecture

This directory contains the architectural design and implementation of the **Decentralized Vaultwarden Secret Rotator**.

## Architecture Diagram

The system has evolved to a decentralized, GitOps-native service.
The design is defined in `architecture.dot`.

```bash
dot -Tpng architecture.dot -o architecture.png
```

![Architecture](./architecture.png)

## Components

- **CLI (`main.rs`)**: The core binary handles scaffolding, discovery, and rotation.
- **Service Worker (CronJob)**: A containerized version runs nightly in K8s, cloning the repo and executing rotation.
- **Modules**:
    - **Discovery**: Scans `apps/` for `rotation.yaml` files.
    - **Scaffold**: Generates new `rotation.yaml` files.
    - **Crypto/Vaultwarden**: Handles zero-knowledge encryption and API interactions.

## Workflows

### 1. Scaffolding (Setup)
Developers create a `rotation.yaml` in their app directory using the CLI:
```bash
cargo make scaffold -- --app my-app --env prod
```
This generates a config file defining which secrets to rotate and how.
It also supports organizing secrets into **Folders** (e.g., "ArgoCD", "Harbor") by specifying a `folder` key in the config. The rotator will automatically create the folder if it doesn't exist.

### 2. Secret Rotation (The Loop)
The rotator (running locally or in the CronJob):
1.  **Scans**: Finds all `rotation.yaml` files in the repository.
2.  **Authenticates**: Logs into Vaultwarden.
3.  **Processes**: For each secret:
    -   Fetches the item from Vaultwarden.
    -   Rotates keys (Random, Static, or File-based).
    -   Updates Vaultwarden with new encrypted data.
    -   Generates a sealed secret (using `kubeseal`).
4.  **Commits**: Pushes the new SealedSecrets to Git.

### 3. Automation
A Kubernetes CronJob runs the rotator nightly.
-   **Manifests**: Defined in `manifests.yaml`.
-   **Docker**: Built via `Makefile.toml`.

## Configuration (`rotation.yaml`)

Each application manages its own configuration:

```yaml
version: v1
secrets:
  - name: my-secret
    vaultwarden:
      cipherId: "..."
      folder: "MyFolder" # New: Organizes secret into this folder
    kubernetes:
      namespace: my-ns
      secretName: my-secret
      path: overlays/prod/sealed-secret.yaml
    keys:
      - name: password
        type: random
        length: 32
      - name: username
        type: static
        value: admin
```

## Developer Guide (Cargo Make)

We use `cargo-make` for common tasks:

| Command | Description |
| :--- | :--- |
| `cargo make check` | Run code checks |
| `cargo make rotate-dry` | Run rotation in dry-run mode |
| `cargo make rotate` | Run actual rotation and commit |
| `cargo make scaffold` | Generate new config |
| `cargo make docker-build` | Build the automation image |
| `cargo make check` | **New**: Run pre-flight health checks |

## Debugging

The CLI supports granular debug flags to help troubleshoot without excessive noise:

- **`--debug`**: General logic and flow logging.
- **`--debug-api`**: Logs full JSON Payloads for Vaultwarden API requests.
- **`--debug-crypto`**: Verbose crypto tracing (MAC, RSA padding).
- **`--debug-auth`**: Auth token details.

Example:
```bash
cargo run -- rotate --scan --debug-api
```
