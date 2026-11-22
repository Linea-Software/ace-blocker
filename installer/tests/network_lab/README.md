# Ace Blocker VPN / TOR network lab — NovaGPT 0.5.x

This framework is **NovaGPT-only**. It controls owned, approved disposable VM
forks using the authenticated NovaGPT **MCP Streamable HTTP** endpoint on
`127.0.0.1`. No alternate VM backend, direct host QEMU control, host route
modification, external MCP endpoint, or fallback is present.

Configuration is **TOML** (`lab.toml`), Python 3.11+ standard library only.
A snapshot identifies an **environment setup** (e.g. `ubuntu-tor-ready`), not a
test case. Dozens of test cases may share an environment snapshot.

## What exists, and what does not

| Function                                                 | Current behavior                                                                                 |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Approved VM images and installation ISO inventory        | Read through `vm_image`, `vm_iso`                                                                |
| Forks with NAT/private NIC, QGA and optional QMP display | `vm(create)` from **human-approved** image names                                                 |
| VM identification and state                              | `vm(status)`; fail closed on wrong ID/image                                                      |
| Environment snapshot save/restore                        | `vm_snapshot`, stopped **legacy-BIOS QEMU QCOW2** only                                           |
| Environment image promotion                              | `vm_snapshot(promote)` for reusable approved bases                                               |
| Guest package provisioning/route checks                  | `vm_guest_status` and `vm_exec` (inside the guest)                                               |
| Guest file transfers                                     | `vm_upload`, `vm_download` (1 MiB maximum)                                                       |
| Guest GUI                                                | `vm_desktop_screenshot`, `vm_desktop_input`                                                      |
| Two-fork WireGuard setup                                 | QEMU private point-to-point NICs plus guest-only WG/nftables                                     |
| Fully installed Linux OS from raw ISO                    | **Not automatic**; installation, QGA and Ace must be provisioned into approved base images first |
| Windows 11 QEMU UEFI/TPM                                 | **Not supported** by the current NovaGPT Linux QEMU fork backend                                 |
| Browser-origin anti-bypass assertions                    | **Not implemented**; marked NOT_RUN, never security PASS                                         |

No subscriptions are required for the owned WireGuard/OpenVPN/TOR tests.
Proton/Windscribe Free, Tor Browser and I2P desktop clients still require a
legitimate human-prepared VM environment. The framework does not create
accounts, bypass authentication or install third-party clients from arbitrary
download URLs.

## 1. Prepare NovaGPT's approved image store

Start from an independently approved, **installed** Linux image with Ace
Blocker, diagnostic dependencies and `qemu-guest-agent` ready. The initial
NovaGPT setup uses human-approved local ISO imports, or a dedicated blank
QCOW2 base, followed by guest OS installation through the VM display.

For Linux **environment snapshots**, use an image installed with legacy BIOS
(`novagpt vm-init-base --legacy` at image preparation time): NovaGPT 0.5.x
correctly refuses disk-only snapshots of UEFI/TPM guests because firmware and
TPM state are not included. This is a deliberate safety limitation.

The image store is operated by the running NovaGPT server. To use human-only
CLI image-import commands, stop that server first to release the exclusive VM
home lease, then restart via the normal `script/dev.py` workflow. Never run
another NovaGPT process against the already locked VM home.

Approve and name the installed base images in the NovaGPT registry, e.g.:

- `ubuntu-ace-ready`
- `arch-ace-ready`
- `cachyos-ace-ready`

The sample `[vms.*].image` entries expect these names; edit them to match
**real approved images**. The `windows-ace-ready` profile is included only for
a compatible NovaGPT Windows-host/Hyper-V workflow; the Linux QEMU path does
not support Windows 11 TPM/UEFI forks and will reject them.

Do not treat an old, separately managed VM as a NovaGPT-managed fork. Only
a validated `vm-<16hex>` ID returned by NovaGPT may be registered.

## 2. Authenticate to the local MCP server

The default TOML uses:

```toml
[lab]
mcp_url = "http://127.0.0.1:8787/mcp"
token_env = "NOVAGPT_HTTP_TOKEN"
artifacts = ".artifacts"
```

Export the authorized **local HTTP MCP token** into the runner process as
`NOVAGPT_HTTP_TOKEN` by your approved secret-delivery mechanism. Do **not**
put it in TOML, source control, the command line, screenshots or reports.
The shell invoking this script is independent of the NovaGPT `dev.py`
supervisor and may not inherit its token automatically. The runner refuses
network access without an explicit environment token. No ngrok or public MCP
endpoint is accepted.

For a **Windows-hosted NovaGPT Hyper-V guest**, `vm_exec` and file transfers
also require the optional operation-scoped secrets `NOVAGPT_GUEST_USERNAME`
and `NOVAGPT_GUEST_PASSWORD`. These are read from the runner's environment
and sent only to NovaGPT's localhost MCP tool. Do not embed them in TOML or
logs. Windows 11 full-VM Hyper-V snapshot management is **not implemented**
by this Python `vm_snapshot` path; it remains unsupported rather than using
an alternate or unsafe snapshot implementation.

The client uses the MCP `initialize` / `notifications/initialized` /
`tools/call` protocol and the operation-specific bounded timeouts.
`--execute` is required for all mutating operations.

## 3. View the configuration, approved images and VM status

From the Ace Blocker repository root:

```bash
python3 installer/tests/network_lab/run.py list
python3 installer/tests/network_lab/run.py inventory --iso
python3 installer/tests/network_lab/run.py doctor --env ubuntu-direct
```

`list` requires no network or credentials. `inventory` and `doctor`
require the local MCP token and live NovaGPT server. `doctor` remains
read-only. If a VM ID is not configured, it prints UNCONFIGURED.

The sample TOML declares **8 VM profiles, 32 environment records and 28
scenarios**. VM profiles include regular Ubuntu/Arch/CachyOS/Windows,
separate Linux WireGuard clients and one common WireGuard gateway. Network
interfaces are declared at fork creation time.

## 4. Create approved disposable forks

First verify the image exists in `inventory`. The following command is a
dry-run without `--execute`:

```bash
python3 installer/tests/network_lab/run.py fork-create --vm ubuntu
python3 installer/tests/network_lab/run.py fork-create --vm ubuntu \
  --execute --confirm-image ubuntu-ace-ready
```

On success the command prints an owned `vm-...` ID. Copy that ID into
`[vms.ubuntu].id` in `lab.toml`. Repeat for each required VM profile.
Do not invent IDs, reuse another project's guest, or place untrusted paths
into the image registry. The TOML parser rejects two VM profiles bound to the
same ID, and the runner verifies NovaGPT's exact fork ID and approved image
before mutation. NovaGPT's current status API does **not** attest which
higher-level test suite originally created a same-image fork: use only IDs
returned for this lab, and review confirmations before destructive restore.

The runner also exposes safe `fork-start` and `fork-stop` actions for
the registered NovaGPT ID (with `--execute --confirm-vm-id`). `fork-stop`
uses NovaGPT's **graceful ACPI shutdown** and does not force-kill the guest.
For example:

```bash
python3 installer/tests/network_lab/run.py fork-stop --env ubuntu-direct \
  --execute --confirm-vm-id vm-0123456789abcdef
python3 installer/tests/network_lab/run.py fork-start --env ubuntu-direct \
  --execute --confirm-vm-id vm-0123456789abcdef
```

The snapshot routines work on the **same owned stopped fork** and its
separately named environmental snapshots. For isolation across concurrent
agents, you may promote a snapshot to an immutable approved image and create
independent forks of that base. The underlying QCOW2 storage remains shared.

## 5. Save an Ace Ready snapshot and derive environments

Assume the `ubuntu` profile has a real ID `vm-0123456789abcdef`; replace
this illustrative value with your own. Before snapshotting, shut the guest
down cleanly through its OS. NovaGPT only accepts the snapshot on a stopped
owned fork.

```bash
# Base image is already installed, QGA-ready and powered off.
python3 installer/tests/network_lab/run.py snapshot-save --env ubuntu-direct \
  --execute --confirm-vm-id vm-0123456789abcdef

# After you boot this fork, provision the TOR package environment.
python3 installer/tests/network_lab/run.py provision --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef

# Shut the guest down normally before saving the TOR-ready environment.
python3 installer/tests/network_lab/run.py snapshot-save --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef
```

To derive another environment from the same parent, restore
`ubuntu-ace-ready` on the **stopped** fork, boot it using NovaGPT, prepare the
new provider and save e.g. `ubuntu-openvpn-ready`. There is **no snapshot
per individual test case**.

```bash
python3 installer/tests/network_lab/run.py snapshot-restore --env ubuntu-direct \
  --execute --confirm-vm-id vm-0123456789abcdef --accept-data-loss

# The run command can restore, start the owned VM and collect bounded probes:
python3 installer/tests/network_lab/run.py run --env ubuntu-tor \
  --restore --execute --confirm-vm-id vm-0123456789abcdef \
  --accept-data-loss --case TOR-03
```

If you need a named reusable approved image for subagents:

```bash
python3 installer/tests/network_lab/run.py snapshot-promote --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef \
  --image ubuntu-tor-ready
```

The promoted immutable image may be supplied to NovaGPT subagents using
`vm_image="ubuntu-tor-ready"`, provided the installed connector exposes
that parameter and the subagent VM manager is enabled. The Python runner
does not initiate subagent jobs.

**Never snapshot an active timed Hard Lock** as a long-lived environment.
Create the Ace policy/Lock after restoring an environment.

## 6. Controlled WireGuard gateway through NovaGPT NICs

NovaGPT exposes `nat`, `listen:segment`, `connect:segment` and `none`
as explicit QEMU NIC configurations. They are **point-to-point links**, not
an Ethernet switch, a VPN service or an isolation guarantee by themselves.

The sample TOML has:

- A `gateway` VM with `nat` and three `listen:ace-*` NICs.
- An `ubuntu-wg`, `arch-wg` or `cachyos-wg` VM with a matching
  `connect:ace-*` NIC.
- Test-only link addresses `172.29.10.1/30 ↔ 172.29.10.2/30` etc.
- Separate inner WireGuard tunnel addresses `10.66.10.1/24 ↔ 10.66.10.2/32` etc.

**Create/start the gateway before any connected client**, or QEMU has no
point-to-point socket to connect to. Register both returned fork IDs in
`[vms.gateway]` and `[vms.ubuntu-wg]` (etc.). Then:

```bash
python3 installer/tests/network_lab/run.py wireguard-setup \
  --env ubuntu-wireguard --execute \
  --confirm-vm-id vm-CLIENT_ID_FROM_NOVAGPT \
  --confirm-gateway-vm-id vm-GATEWAY_ID_FROM_NOVAGPT
```

Both example ID strings above are placeholders; supply the actual
`vm-<16hex>` values. The gateway must be a separate running, owned fork
with working QGA. It is provisioned through `vm_exec`: keys are generated
and kept inside the guest, and guest nftables/NAT rules are applied to the
gateway **only**, never to the host. Private Ethernet link addresses are
persisted in guest systemd units so that a snapshot restored after reboot
does not silently lose the VPN transport. The gateway's wg-quick unit
depends on the private link unit. A configured WireGuard client remains
down until `connect`. Capture a provider-ready snapshot after shutting
down normally.

```bash
python3 installer/tests/network_lab/run.py connect --env ubuntu-wireguard \
  --execute --confirm-vm-id vm-CLIENT_ID_FROM_NOVAGPT
```

**Important limitation:** These example client VMs have **both a NAT NIC
and a private point-to-point NIC**. NAT may offer a bypass path when the
tunnel goes down. Do **not** call this configuration a no-leak or kill-switch
test. For a stronger leak experiment, prepare a separate client image with
required packages already installed and fork it using _only_ the private
`connect:*` NIC. Validate DNS, IPv6 and origin-server logs independently.
This WireGuard fixture is IPv4-only.

An OpenVPN lab client can be prepared with a reviewed, owned
`/etc/openvpn/client/ace-lab.conf`, but gateway PKI/setup is not automated
by this framework.

## 7. Diagnostics and GUI

```bash
python3 installer/tests/network_lab/run.py run --env ubuntu-tor \
  --case TOR-03 --execute --confirm-vm-id vm-0123456789abcdef \
  --require-coverage

python3 installer/tests/network_lab/run.py screenshot --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef

python3 installer/tests/network_lab/run.py keys --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef --key ctrl --key l

python3 installer/tests/network_lab/run.py pointer --env ubuntu-tor \
  --execute --confirm-vm-id vm-0123456789abcdef \
  --x 12000 --y 9000 --button left
```

The `upload` and `download` actions require explicit absolute _guest_
paths, remain limited to 1 MiB per call and use the NovaGPT Guest Agent.
Downloaded files and screenshots are written in the private `.artifacts/`
folder with restrictive permissions; they are not committed to Git.

## 8. Result meanings and release criteria

Each run produces a mode-0600 JSON report with the approved image, NovaGPT
fork ID, snapshot name, Git commit and per-case result.

- `INFRA_OK`: read-only guest route/service/listener probe completed.
- `INFRA_FAILED`: probe failed or output was truncated.
- `NOT_RUN`: native browser/transport/provider integration is missing.
- `BLOCKED`: NovaGPT guest interaction failed or was unavailable.

A suite with only `NOT_RUN` cases exits 3. The `--require-coverage`
option exits nonzero if **any** selected case remains unexecuted.
`INFRA_OK` never means that Ace Blocker stopped a browser bypass. A true
security PASS requires correlated evidence from the browser, Ace Blocker
enforcement decision, and controlled blocked/allowed HTTPS origin servers.
No such browser-origin verifier is implemented in this lab yet.

Real Proton/Windscribe sessions must remain private. Do not commit account
tokens, downloaded VPN configs, bridge credentials, browser profiles,
private keys or private screenshots.

## 9. Offline checks

```bash
python3 -m unittest installer.tests.network_lab.test_framework -v
python3 -m compileall -q installer/tests/network_lab
python3 installer/tests/network_lab/run.py list
python3 installer/tests/network_lab/run.py run --env ubuntu-tor --case TOR-03
```

All commands lacking `--execute` are plans, never guest mutations.
