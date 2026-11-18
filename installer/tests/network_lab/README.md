# Ace Blocker network lab (Linux / Windows)

This **stdlib-only Python** framework manages *environment snapshots*, not one
snapshot per test ID. It extends the existing libvirt/QEMU guest-agent harness.
Configuration is **TOML** (`lab.toml`); no YAML is required.

## Supported operations

- Validate TOML environments and scenarios, show a read-only plan and inspect VM status.
- Provision **official distro packages** for WireGuard, OpenVPN, TOR daemon,
  SSH/SOCKS fixtures and diagnostic tools in Ubuntu, Arch and CachyOS.
- Activate/deactivate a preconfigured `ace-lab` WireGuard/OpenVPN interface or
  TOR service in a disposable Linux guest.
- Save/revert an **offline internal libvirt snapshot** for each environment.
  Multiple snapshots may share one base VM/domain per OS, e.g. `ubuntu-tor-ready`,
  `ubuntu-wireguard-ready`, `ubuntu-proton-ready`.
- Run bounded QEMU guest-agent diagnostics and write private JSON evidence.
  Browser-origin security scenarios remain `NOT_RUN` until independently tested.

**Not included yet:** real-browser automation, automatic creation of OpenVPN PKI,
TOR bridges, licensed VPN provider credentials and automatic Tor Browser/I2P
installation. A two-VM WireGuard gateway/client fixture *is* supported, but
requires an owned gateway VM with a reachable address configured in TOML.
Never call security tests PASS based only on a service check, active VPN route
or listening SOCKS port.

## Prerequisites

Install `virsh`, `qemu-img`, libvirt and QEMU on the **host**, with explicitly
imported disposable QCOW2-backed domains. Enable `qemu-guest-agent` on each
guest image. In `lab.toml`, change `ace-lab-ubuntu`, `ace-lab-arch`,
`ace-lab-cachyos` and `ace-lab-windows11` to **your owned test VMs**.
The sample names are placeholders: they do not create a VM.

The VM must be **shut off** to create or revert snapshots. The framework
verifies an exact VM UUID, `ace-lab-*` disposable name, single file-backed
QCOW2 disk, offline internal snapshot state and matching snapshot disk metadata.
It **rejects** UEFI NVRAM, virtual TPM, external snapshots, host file-sharing
devices and multi-disk layouts: libvirt internal disk snapshots do not capture
all those machine states. **Windows 11 with TPM/OVMF must use a separately
validated full-VM snapshot backend**; this initial safe snapshot implementation
cannot manage such Windows snapshots. The Windows guest-agent probes can still
run on a manually prepared live VM without `--restore`.
Make the `*-ready` snapshots only after configuration, without active timed
Locks. Create fresh test Locks during each run.

## Snapshot branching: one OS VM, many provider-ready states

Treat the `*-direct` profile as the **Ace Ready parent snapshot**
(`ubuntu-ace-ready`, `arch-ace-ready`, `cachyos-ace-ready` or
`windows-ace-ready` in the sample TOML). Provision Ace on the common OS
image, shut down cleanly and save that parent once.

For each provider, first restore the **parent** snapshot on a shut-off VM
(`snapshot-restore --env ubuntu-direct --execute --confirm-domain ace-lab-ubuntu
--confirm-uuid UUID_FROM_VIRSH --accept-data-loss`), boot the VM,
provision just that provider, shut it
down and save the corresponding provider snapshot (for example
`snapshot-save --env ubuntu-tor`). Repeat from the **same parent** for
WireGuard, OpenVPN, TOR, I2P and others. Do not build later provider snapshots
on top of a previous provider's modified state. The snapshot chain can share
the same QCOW2 storage; no snapshot is required for each individual test case.

## CLI

Run from the repository root:

```bash
python3 installer/tests/network_lab/run.py list
python3 installer/tests/network_lab/run.py doctor --env ubuntu-tor

# On a running, disposable, guest-agent-ready Ubuntu test VM:
python3 installer/tests/network_lab/run.py provision --env ubuntu-tor
python3 installer/tests/network_lab/run.py provision --env ubuntu-tor \
  --execute --confirm-domain ace-lab-ubuntu

# Once the guest has been cleanly shut down without active Locks:
# Obtain the UUID with: virsh -c qemu:///session domuuid ace-lab-ubuntu
python3 installer/tests/network_lab/run.py snapshot-save --env ubuntu-tor \
  --execute --confirm-domain ace-lab-ubuntu --confirm-uuid UUID_FROM_VIRSH

# Explicit destructive restore and suite execution, VM initially shut off:
python3 installer/tests/network_lab/run.py run --env ubuntu-tor \
  --restore --accept-data-loss --execute --confirm-domain ace-lab-ubuntu \
  --confirm-uuid UUID_FROM_VIRSH

# Or run only non-destructive infrastructure probes on a running guest:
python3 installer/tests/network_lab/run.py run --env ubuntu-tor \
  --case TOR-03 --execute --confirm-domain ace-lab-ubuntu
```

`--execute` is absent by default; planning does not contact or change a VM.
`--confirm-domain` must exactly match the TOML's libvirt domain.
Restore also requires `--confirm-uuid`, `--accept-data-loss` and a shut-off VM.
The framework never
uses `virsh destroy`, `snapshot-revert --force`, or bypasses Ace Lock gates.

### Controlled WireGuard gateway (optional)

Set `[wireguard_lab]` in TOML to a reachable **disposable gateway VM** and set
its actual IPv4 endpoint. The gateway and client must both be running with a
working QEMU Guest Agent. Confirm both domain names separately:

```bash
python3 installer/tests/network_lab/run.py wireguard-setup \
  --env ubuntu-wireguard --execute \
  --confirm-domain ace-lab-ubuntu \
  --confirm-gateway-domain ace-lab-vpn-gateway \
  --confirm-gateway-uuid GATEWAY_UUID_FROM_VIRSH
```

Before first setup, shut down the owned gateway and save
`snapshot-save --env wireguard-gateway --execute
--confirm-domain ace-lab-vpn-gateway --confirm-uuid GATEWAY_UUID_FROM_VIRSH`
against its **clean disposable** base. Inspect the gateway UUID with
`virsh -c qemu:///session domuuid ace-lab-vpn-gateway`.

The command installs official WireGuard/nftables packages in both guests,
generates private keys inside them, configures independent gateway interfaces
(`acegw0`, `acegw1`, `acegw2`) and enables NAT/IP forwarding **inside the
gateway guest only**. This fixture currently tests **IPv4 only**; dual-stack
IPv6 paths must be checked separately. Save/restore the dedicated
`wireguard-gateway` base snapshot when you need a clean gateway, since
forwarding is intentionally enabled in that disposable VM and is not
automatically restored to its pre-lab sysctl value.
It creates the client configuration but does not connect
the client. Save the client provider-ready snapshot after setup; the same
snapshot can run multiple test IDs. Do **not** run this against your laptop or
a shared/production VM. External network topology and firewall rules still
need native verification, and a working tunnel is not proof of blocking.

For WireGuard, the optional `wireguard-setup` command can generate test-only
keys **inside the client and gateway VMs**, create
`/etc/wireguard/ace-lab.conf` and configure an owned Linux VPN gateway.
The guest private keys never enter Git or the host-side report. It leaves the
client tunnel disconnected until explicitly started. For OpenVPN, install the
client and supply a manually reviewed `/etc/openvpn/client/ace-lab.conf`
pointing to an owned lab peer. Capture a provider-ready snapshot after setup.
No customer credentials or commercial VPN configurations are included.

```bash
python3 installer/tests/network_lab/run.py connect --env ubuntu-wireguard \
  --execute --confirm-domain ace-lab-ubuntu
python3 installer/tests/network_lab/run.py disconnect --env ubuntu-wireguard \
  --execute --confirm-domain ace-lab-ubuntu
```

Proton and Windscribe Free must be prepared **manually** on private disposable
snapshots after legitimate account login. Never store account credentials,
tokens or provider state in this repository. Tor Browser, I2P and TOR bridges
are also manual/fixture-based for now; automated daemon package setup is not
a verified Tor Browser installation.

## Security test evidence

The tool writes a mode-0600 JSON report under `.artifacts/` (ignored by Git)
with environment, snapshot name, Git commit, probe output and status.

- `INFRA_OK`: a read-only guest service/route/proxy probe succeeded.
- `INFRA_FAILED`: a probe returned a nonzero exit code.
- `NOT_RUN`: browser/TOR-bridge/VPN provider scenario needs real test evidence.
- `BLOCKED`: VM/guest-control infrastructure unavailable.

An all-`NOT_RUN` suite exits with code 3. Use `run --require-coverage` to
exit nonzero if **any** requested case was not executed. Infrastructure
success is never a browser-enforcement PASS; use those results only to assess
readiness, not as a security release gate.

**None of these statuses means end-to-end ACE enforcement passed.** A real
PASS requires browser evidence from a protected browser process **and** the
controlled blocked/allowed HTTPS origins, correlated with ACE
TUN/WFP/nftables decisions. Use network captures only in private disposable
guests, and redact IPs, cookies, proxy passwords and capability tokens before
sharing evidence.

## Offline validation

```bash
python3 -m unittest installer.tests.network_lab.test_framework -v
python3 -m compileall -q installer/tests/network_lab
python3 installer/tests/network_lab/run.py run --env ubuntu-tor --case TOR-03
```
