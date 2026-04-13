#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "This script must be run as root." >&2
  exit 1
fi

if [[ "$#" -lt 1 || "$#" -gt 2 ]]; then
  echo "Usage: $0 <user-ca-public-key> [destination-path]" >&2
  exit 2
fi

SOURCE_KEY="$1"
DESTINATION_KEY="${2:-/etc/ssh/agent-ssh-user-ca.pub}"
SSHD_CONFIG_PATH="${SSHD_CONFIG_PATH:-/etc/ssh/sshd_config}"

if [[ ! -f "${SOURCE_KEY}" ]]; then
  echo "CA public key not found: ${SOURCE_KEY}" >&2
  exit 3
fi

install -m 0644 "${SOURCE_KEY}" "${DESTINATION_KEY}"

if ! grep -Fqx "TrustedUserCAKeys ${DESTINATION_KEY}" "${SSHD_CONFIG_PATH}"; then
  cat <<EOF >> "${SSHD_CONFIG_PATH}"

# agent-ssh user CA trust
TrustedUserCAKeys ${DESTINATION_KEY}
# Optional hardening:
# AuthorizedPrincipalsFile /etc/ssh/auth_principals/%u
EOF
fi

if command -v sshd >/dev/null 2>&1; then
  sshd -t
fi

if command -v systemctl >/dev/null 2>&1; then
  systemctl reload sshd || systemctl reload ssh
else
  echo "Reload sshd manually for changes to take effect." >&2
fi
