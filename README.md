# resolved-nftset

A Linux service that monitors DNS queries resolved via `systemd-resolved` and adds IPs matching specific hostnames to `nftables` sets. This enables policy-based routing — for example, routing traffic to certain domains through a WireGuard VPN.

```
systemd-resolved  ──varlink──▶  resolved-nftset  ──netlink──▶  nftables kernel sets
```

## How it works

1. Connects to `systemd-resolved` via the `io.systemd.Resolve.Monitor` varlink interface.
2. Listens for all DNS responses in real time.
3. Matches each resolved hostname against user-defined rules (FQDN prefix matching).
4. On a match, adds the resolved IPv4/IPv6 addresses to the corresponding nftables set.
5. Your nftables ruleset marks packets destined for those IPs, and routing policy sends them to your desired interface.

## Requirements

- **systemd ≥ 246** (for `io.systemd.Resolve.Monitor`)
- **nftables**

## Installation

### Build from source

```bash
cargo build --release
sudo install -m 755 target/release/resolved-nftset /usr/local/bin/
```

### Install the systemd service

```bash
sudo useradd -r -s /sbin/nologin resolved-nftset
sudo install -m 644 resolved-nftset.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now resolved-nftset
```

## Configuration

Configuration lives under `/etc/resolved-nftset/`. The directory structure maps to the nftables table and set names:

```
/etc/resolved-nftset/
└── <table_name>/           # nftables table name
    └── <set_name>/         # set name (without _v4, _v6 suffix)
        └── rules.conf      # hostnames to match, one per line
```

### Example: route VPN traffic

Suppose you want traffic to `google.com` and `youtube.com` to go through a WireGuard interface (`wg0`).

#### 1. Create the rule file

```bash
sudo mkdir -p /etc/resolved-nftset/resolved_route/vpn
printf 'google.com\nyoutube.com\n' | sudo tee /etc/resolved-nftset/resolved_route/vpn/rules.conf
```

#### 2. Load the nftables ruleset

Create `/etc/nftables/resolved-mark.nft`:

```nft
table inet resolved_mark {
    set vpn_v4 {
        type ipv4_addr
        flags timeout
        timeout 1h
        gc-interval 5m
    }

    set vpn_v6 {
        type ipv6_addr
        flags timeout
        timeout 1h
        gc-interval 5m
    }

    chain prerouting {
        type filter hook prerouting priority mangle; policy accept;

        ip  daddr @vpn_v4 meta mark set 0xFF
        ip6 daddr @vpn_v6 meta mark set 0xFF
    }
}
```

Load it:

```bash
sudo install -m 640 resolved-mark.nft /etc/nftables/resolved-mark.nft
sudo nft -f /etc/nftables/resolved-mark.nft
```

Add `include "/etc/nftables/resolved-mark.nft"` to your main nftables config.

#### 3. Add policy routing rules

Mark `0xFF` (255) needs a routing rule that directs marked packets to the VPN:

```bash
# Create a routing table for VPN traffic (table number 100 is arbitrary)
echo "100 vpn" | sudo tee -a /etc/iproute2/rt_tables

# Packets marked 0xFF go through table "vpn"
sudo ip rule add fwmark 0xFF table vpn

# All routes in the "vpn" table go out via wg0
sudo ip route add default dev wg0 table vpn
```

#### 4. Start the service

```bash
sudo systemctl restart resolved-nftset
```

Now any DNS resolution for `google.com` or `youtube.com` will cause the daemon to add the resolved IPs to the `vpn_v4`/`vpn_v6` sets. Packets to those IPs get marked `0xFF` in prerouting, and the policy routing rule sends them through `wg0`.

### Multiple sets

You can define multiple rule directories for different routing policies:

```
/etc/resolved-nftset/
├── resolved_route/
│   ├── vpn/
│   │   └── rules.conf      # → vpn_v4, vpn_v6 sets → route via wg0
│   └── tor/
│       └── rules.conf      # → tor_v4, tor_v6 sets → route via tor
└── resolved_block/
    └── ads/
        └── rules.conf      # → ads_v4, ads_v6 sets → drop in filter chain
```

### Rules file format

- One hostname per line
- Lines starting with `#` are comments
- Blank lines are ignored
- Matching is prefix-based (e.g., `google.com` matches `www.google.com`, `mail.google.com`, etc.)

## Security notes

- The service requires `CAP_NET_ADMIN` to write to nftables via Netlink.
- The systemd unit uses `AmbientCapabilities=CAP_NET_ADMIN` so it can run as an unprivileged user.
- All other systemd sandboxing options (`ProtectSystem=strict`, `MemoryDenyWriteExecute`, etc.) are enabled by default.

## License

MIT OR Apache-2.0
