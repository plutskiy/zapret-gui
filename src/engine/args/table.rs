pub(super) struct OptSpec {
    pub(super) name: &'static str,
    pub(super) kind: u8, // 0 none, 1 required, 2 optional
}

pub(super) const OPTS: &[OptSpec] = &[
    OptSpec {
        name: "debug",
        kind: 2,
    },
    OptSpec {
        name: "dry-run",
        kind: 0,
    },
    OptSpec {
        name: "version",
        kind: 0,
    },
    OptSpec {
        name: "comment",
        kind: 2,
    },
    OptSpec {
        name: "qnum",
        kind: 1,
    },
    OptSpec {
        name: "daemon",
        kind: 0,
    },
    OptSpec {
        name: "pidfile",
        kind: 1,
    },
    OptSpec {
        name: "user",
        kind: 1,
    },
    OptSpec {
        name: "uid",
        kind: 1,
    },
    OptSpec {
        name: "wsize",
        kind: 1,
    },
    OptSpec {
        name: "wssize",
        kind: 1,
    },
    OptSpec {
        name: "wssize-cutoff",
        kind: 1,
    },
    OptSpec {
        name: "wssize-forced-cutoff",
        kind: 1,
    },
    OptSpec {
        name: "synack-split",
        kind: 2,
    },
    OptSpec {
        name: "ctrack-timeouts",
        kind: 1,
    },
    OptSpec {
        name: "ctrack-disable",
        kind: 2,
    },
    OptSpec {
        name: "ipcache-lifetime",
        kind: 1,
    },
    OptSpec {
        name: "ipcache-hostname",
        kind: 2,
    },
    OptSpec {
        name: "hostcase",
        kind: 0,
    },
    OptSpec {
        name: "hostspell",
        kind: 1,
    },
    OptSpec {
        name: "hostnospace",
        kind: 0,
    },
    OptSpec {
        name: "domcase",
        kind: 0,
    },
    OptSpec {
        name: "methodeol",
        kind: 0,
    },
    OptSpec {
        name: "ip-id",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fwmark",
        kind: 1,
    },
    OptSpec {
        name: "dup",
        kind: 1,
    },
    OptSpec {
        name: "dup-ttl",
        kind: 1,
    },
    OptSpec {
        name: "dup-ttl6",
        kind: 1,
    },
    OptSpec {
        name: "dup-autottl",
        kind: 2,
    },
    OptSpec {
        name: "dup-autottl6",
        kind: 2,
    },
    OptSpec {
        name: "dup-tcp-flags-set",
        kind: 2,
    },
    OptSpec {
        name: "dup-tcp-flags-unset",
        kind: 2,
    },
    OptSpec {
        name: "dup-fooling",
        kind: 1,
    },
    OptSpec {
        name: "dup-ts-increment",
        kind: 1,
    },
    OptSpec {
        name: "dup-badseq-increment",
        kind: 1,
    },
    OptSpec {
        name: "dup-badack-increment",
        kind: 1,
    },
    OptSpec {
        name: "dup-replace",
        kind: 2,
    },
    OptSpec {
        name: "dup-ip-id",
        kind: 1,
    },
    OptSpec {
        name: "dup-start",
        kind: 1,
    },
    OptSpec {
        name: "dup-cutoff",
        kind: 1,
    },
    OptSpec {
        name: "orig-ttl",
        kind: 1,
    },
    OptSpec {
        name: "orig-ttl6",
        kind: 1,
    },
    OptSpec {
        name: "orig-autottl",
        kind: 2,
    },
    OptSpec {
        name: "orig-autottl6",
        kind: 2,
    },
    OptSpec {
        name: "orig-tcp-flags-set",
        kind: 2,
    },
    OptSpec {
        name: "orig-tcp-flags-unset",
        kind: 2,
    },
    OptSpec {
        name: "orig-mod-start",
        kind: 1,
    },
    OptSpec {
        name: "orig-mod-cutoff",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-ttl",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-ttl6",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-autottl",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-autottl6",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-tcp-flags-set",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-tcp-flags-unset",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-fooling",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-repeats",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-skip-nosni",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-split-pos",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-split-http-req",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-split-tls",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-split-seqovl",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-split-seqovl-pattern",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fakedsplit-pattern",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fakedsplit-mod",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-hostfakesplit-midhost",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-hostfakesplit-mod",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-ipfrag-pos-tcp",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-ipfrag-pos-udp",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-ts-increment",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-badseq-increment",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-badack-increment",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-any-protocol",
        kind: 2,
    },
    OptSpec {
        name: "dpi-desync-fake-tcp-mod",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-http",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-tls",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-tls-mod",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-unknown",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-syndata",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-quic",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-wireguard",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-dht",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-discord",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-stun",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-fake-unknown-udp",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-udplen-increment",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-udplen-pattern",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-cutoff",
        kind: 1,
    },
    OptSpec {
        name: "dpi-desync-start",
        kind: 1,
    },
    OptSpec {
        name: "hostlist",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-domains",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-exclude",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-exclude-domains",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-auto",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-auto-fail-threshold",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-auto-fail-time",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-auto-retrans-threshold",
        kind: 1,
    },
    OptSpec {
        name: "hostlist-auto-debug",
        kind: 1,
    },
    OptSpec {
        name: "new",
        kind: 0,
    },
    OptSpec {
        name: "skip",
        kind: 0,
    },
    OptSpec {
        name: "filter-l3",
        kind: 1,
    },
    OptSpec {
        name: "filter-tcp",
        kind: 1,
    },
    OptSpec {
        name: "filter-udp",
        kind: 1,
    },
    OptSpec {
        name: "filter-l7",
        kind: 1,
    },
    OptSpec {
        name: "ipset",
        kind: 1,
    },
    OptSpec {
        name: "ipset-ip",
        kind: 1,
    },
    OptSpec {
        name: "ipset-exclude",
        kind: 1,
    },
    OptSpec {
        name: "ipset-exclude-ip",
        kind: 1,
    },
    OptSpec {
        name: "bind-fix4",
        kind: 0,
    },
    OptSpec {
        name: "bind-fix6",
        kind: 0,
    },
    OptSpec {
        name: "wf-iface",
        kind: 1,
    },
    OptSpec {
        name: "wf-l3",
        kind: 1,
    },
    OptSpec {
        name: "wf-tcp",
        kind: 1,
    },
    OptSpec {
        name: "wf-udp",
        kind: 1,
    },
    OptSpec {
        name: "wf-raw",
        kind: 1,
    },
    OptSpec {
        name: "wf-raw-part",
        kind: 1,
    },
    OptSpec {
        name: "wf-filter-lan",
        kind: 1,
    },
    OptSpec {
        name: "wf-save",
        kind: 1,
    },
    OptSpec {
        name: "ssid-filter",
        kind: 1,
    },
    OptSpec {
        name: "nlm-filter",
        kind: 1,
    },
];
