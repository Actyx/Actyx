---
title: Fixing an interop issue in libp2p gossipsub
author: Rüdiger Klaehn
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/rklaehn
author_image_url: /images/blog/ruediger-klaehn.jpg
tags: [IPFS rust libp2p dweb]
---

We are currently migrating our core event dissemination system to pure rust. In the process of doing so, we have discovered an interop issue between go-libp2p and rust-libp2p.

This post describes the process of finding and fixing the issue.

<!-- truncate -->

## Libp2p

The [libp2p](https://libp2p.io/) network stack is a core component of many recent distributed web and blockchain projects. It is developed for the [polkadot](https://polkadot.network/) blockchain, but is also going to be the networking stack of [ethereum 2](https://github.com/ethereum/eth2.0-specs/blob/dev/specs/phase0/p2p-interface.md#network-fundamentals).

At Actyx, we are using libp2p as the peer to peer networking stack for ActyxOS, most notably for our partition tolerant event dissemination system.

Until now, we have been spawning an ipfs process to take advantage of libp2p. While this works well, it has some overhead that is no longer acceptable for us as the size of our production installations and the demands of our system integrator customers increases. So in the past months we have been migrating to a pure rust solution, using the rust implementation of libp2p that is developed by parity.

This will allow us to dramatically reduce the size and complexity of ActyxOS binaries while drastically improving performance. As an example: the size of the ActyxOS apk changes from <value> to <value>.

## Putting things in production

I have tested interop with go-ipfs extensively in various scenarios over the last weeks. So last week it was finally time to push this into production at our customers.

Prior to putting it into production, we did *one last check* with a different go-ipfs node than the one we had been using for testing.

And suddenly, **nothing** worked. Our node was unable to get any events from the go-ipfs node.

The latest version of our event dissemination system relies on the gosspsub and the bitswap protocol of libp2p. Since we are using our own custom bitswap implementation, the first thought was that that this might be the cause of the issue. However, further investigation revealed that the new node was not even able to get any message via gossipsub.

Initial attempts to solve the issue got me on the wrong track. It turned out that at least on my machine, everything worked just fine when I added a small delay between the peer connection and the subscription to the gossipsub topic. So I thought this might be a race condition in the handshake process between two nodes. But further tests by my colleagues revealed that this delay did not always solve the problem.

At this point I was a bit lost. I created an [issue](https://github.com/libp2p/rust-libp2p/issues/1671) in rust-libp2p and hoped that somebody would help me looking into this.

## Open source to the rescue

Thankfully, Adrian Manning from [sigma prime](https://sigmaprime.io/), the developers of the [lighthouse](https://lighthouse.sigmaprime.io/) Ethereum 2.0 client, immediately jumped on the issue. Just the fact that somebody other than me was willing to help fixing this issue was a huge relief.

Adrian is the main author of the rust implementation of gossipsub, which is a [central component](https://github.com/ethereum/eth2.0-specs/blob/dev/specs/phase0/p2p-interface.md#the-gossip-domain-gossipsub) of Ethereum 2.

Once I looked at the tracing output at the finest level to get Adrian some info, the cause of the issue became clear relatively quickly. Our production node was go-ipfs 0.4.22. It was sending us messages that did not conform to the protocol buffers specification of the gossipsub protocol. The [messageids field](https://github.com/libp2p/go-libp2p-pubsub/blob/dd069798bb31b4e79f7222e7a72d922695537d7b/pb/rpc.proto#L35), that was specified as a string in the protocol buffers definition in go-ipfs, was sometimes not a valid utf8 string.

## Enter protocol labs

Since this was clearly something to do with go-ipfs, we had to get [protocol labs](https://protocol.ai/) involved. Protocol labs are the original developers of the libp2p spec, including the [gossipsub spec](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub).

Again, the response was very quick and helpful.

After only a few hours I got in contact with Dimitris Vyzovitis, one of the main authors of the gossipsub spec. Protocol labs confirmed that this was an issue on the go-libp2p side. It turns out that the protocol buffers library that comes with go allows both emitting and reading non-uft8 values for string fields in protocol buffers definitions, which is not according to the spec:
```
A string must always contain UTF-8 encoded or 7-bit ASCII text, and cannot be longer than 2³².
```

This has been [fixed](https://github.com/golang/protobuf/issues/484) in the latest version of the library, but the fact remains that there are lots of go-ipfs nodes in production that emit non utf8 message ids.

## Changing the spec

Message ids are opaque message identifiers to allow the gossipsub system to keep track of messages. There is no benefit in having them be human readable utf8 strings. In fact, in many cases these strings are just completely random strings.

So a decision was made quickly to adjust the spec and put a warning into the go-ipfs protocol buffers specification.

The new specification is an improvement over the old one. For one, it matches reality. But more importantly, using bytes for message ids is the right thing to do. Often it is convenient to generate globally unique message ids by concatenating the peer id (a hash) and a counter or a sufficiently large random number. Previously, this data would have to be base64 encoded to make it a valid utf8 string. This makes the protocol less efficient while not making it any more human readable.

## Fallout

After updating the spec, I made a pull request against rust-libp2p to change the message id from a rust `String` to a rust `Vec<u8>`, which can hold an arbitrary sequence of bytes. After this change, connection to our production go-ipfs node immediately worked without the delay hack.

Go-ipfs will have to adjust their code generation and eventually change the protobuf definition in their repo to match the new spec. But until then, there is now going to be [a warning](https://github.com/libp2p/go-libp2p-pubsub/pull/363) as a comment in the .proto file.

Js-ipfs will have to make sure to properly handle non-utf8 message ids. I have created [an issue](https://github.com/libp2p/js-libp2p-pubsub/issues/67) in their repo to make them aware of the problem. However, non utf8 message ids been there all along, and go-libp2p / js-libp2p interop has been extensively tested. So things might already work.

## Zooming out

This was a view inside how open source software is developed. A view inside the sausage factory, if you will.

As a library user or end user, the bottom line is that the interop between go-libp2p and rust-libp2 has been improved. This gets us closer to the goal of libp2p as a language independent network stack for peer to peer applications.

For actyx, this fix means that we are unblocked to release ActyxOS 1.0 with significantly reduced memory footprint and improved performance.

Stay tuned!