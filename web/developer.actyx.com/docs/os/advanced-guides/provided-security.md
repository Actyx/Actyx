---
title: Security Provided by ActyxOS
---

This page details basic network security guarantees provided by ActyxOS.

## Peer-to-Peer Communication

Peer-to-peer communication has multiple layers of encryption.
The most basic one is based on a Pre-shared Key, which ensures that only ActyxOS nodes configured to be in the same swarm can connect to each other.
The mechanism employed here is based on this [libp2p specification](https://github.com/libp2p/specs/blob/master/pnet/Private-Networks-PSK-V1.md) using the [Salsa20 stream cipher](https://en.wikipedia.org/wiki/Salsa20).

:::info
This means, that whoever is in possession of said PSK is able to read all events published in a swarm.
We're currently working on per-stream encryption, which will yield high granularity in permissions.
:::

Every connection between ActyxOS nodes is further encrypted using elliptic-curive cryptography (specifically Ed25519, using SHA2 and Curve25519).
This means that every ActyxOS node holds an ed25519 key pair.
That key pair is currently saved to an encrypted file on disk, we plan to add TPM-backed key stores in the future.

## Interaction with an ActyxOS node using the Actyx CLI

Interacting directly with an ActyxOS node is straight-forward using the [Actyx CLI](../../cli/ax), which interacts with an ActyxOS node directly using a HTTP-based API.
This API usage provides neither authentication nor encryption, and should only be used in secure network environments (e.g. development setup, and restricted production installations).

However, we offer an experimental feature, which replaces said HTTP-based API with one, which functions quite similar to
the widely known program SSH, making use of the key pair available on every ActyxOS node.

:::info
This experimental API is listening on TCP port 4458, and can be accessed by providing a
so called _multiaddress_ to the `ax` command:

* `localhost:4458` becomes either `/dns4/localhost/tcp/4458` or `/ip4/127.0.0.1/tcp/4458`
* `192.168.100.42:4458` becomes `/ip4/192.168.100.42/tcp/4458`

(this API only supports IPv4 at the moment)
:::

To interact with a remote node, first a local key pair authenticating the client needs to be created.
This is done automatically using the `ax` command:

```
➜  ~ ax settings get . --local /dns4/localhost/tcp/4458
No local key pair found, generated one ("0hoWXFyVHzbhhQ6fU+z/kZb8oaN+ligosj2/hzuJn04I=") at location /home/dev/.config/actyxos/keys
```

Interacting with the remote host will use this key pair:

```
➜  ~ ax settings get . --local /dns4/localhost/tcp/4458
The authenticity of host '/dns4/localhost/tcp/4458' can't be established.
Node's ID is '12D3KooWH4duqeY7Pj3or9j7aKnmaxz8WUYtVYnZUbACW21hGvBh'.
Are you sure you want to continue connecting (yes/no)?
```

As the client has not yet established any connection with that host, its authenticity needs to be proven.
After this information is persisted to disk, any further interactions guarantee the authenticity.
If the remote ActyxOS node's identity changes, a warning is displayed, and any further interactions blocked until the user verifies the new identity.

This mechanism works also in the other direction, where the public key the Actyx CLI is using to authenticate against the remote ActyxOS node can be added to the `authorizedKeys` settings of the remote node.
After this has been done, the ActyxOS node can be configured to only accept connections, that can prove to have this identity.
And the open HTTP API will be disabled.
Let's see how that looks like in practice.
First, let's find out the local key id:

```
ax swarms listKeys
Local keystore location: /home/ow/.config/actyxos/keys

Available key pairs (identified by the respective public key):
0hoWXFyVHzbhhQ6fU+z/kZb8oaN+ligosj2/hzuJn04I=
```

Then our public key is added to the ActyxOS node.

```
ax settings set com.actyx.os/general/authorizedKeys ["0hoWXFyVHzbhhQ6fU+z/kZb8oaN+ligosj2/hzuJn04I="] --local /dns4/localhost/tcp/4458
Successfully replaced settings at com.actyx.os/general/authorizedKeys. Created object with defaults:
---
general:
  announceAddresses: []
  authorizedKeys:
    - 0hoWXFyVHzbhhQ6fU+z/kZb8oaN+ligosj2/hzuJn04I=
# ..
```

Afterwards, we can switch on the `requireAuthentication` flag to reject all unauthorized access using

```
ax settings set com.actyx.os/general/requireAuthentication true --local /dns4/localhost/tcp/4458
```

Now, every interaction between the Actyx CLI and the remote ActyxOS node is authenticated, authorized, and encrypted.

Note that interfacing with the HTTP-based API won't work any longer:

```
ax settings get . --local localhost
[ERR_NODE_MISCONFIGURED] Error: Node HTTP API is not available, if setting `com.actyx.os/general/requireAuthentication` is set to true. Consider using the experimental API on port 4458 instead.
```

:::info
This feature is still experimental and subject to change.
:::
