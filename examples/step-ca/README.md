# step-ca Example Notes

This directory documents the intended direction for a future `step-ca` signer implementation.

## Design Intent

- Use `step-ca` as a named signer backend selected through the broker configuration.
- Issue short-lived SSH user certificates for broker-mediated sessions.
- Configure remote hosts to trust the SSH user CA via `TrustedUserCAKeys`.
- Optionally restrict accepted principals with `AuthorizedPrincipalsFile`.

## Important Boundary

The foundation milestone defines the signer abstraction and configuration model, but it does not yet claim a complete production-ready `step-ca` execution path.

## Server Trust Bootstrap

The helper script [`scripts/install-server-ca-trust.sh`](/Users/aibunny/agent-ssh/scripts/install-server-ca-trust.sh) installs a user CA public key into `sshd_config` using `TrustedUserCAKeys`, which follows the documented OpenSSH certificate trust model.
