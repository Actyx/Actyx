# Representing IPFS links in events

|  |  |
| --- | --- |
| date | 2020-05-25 |
| status | accepted |
| persons | @rkuhn, @mhaushofer, @nicolas-actyx |

## Decision

We will in a future Actyx version recognise IPFS links in event payloads.
If the payload arrives in CBOR encoding, this is done by noticing byte strings with tag 42 (as per the IPLD standard).
If the payload arrives in JSON encoding, we recognise the IPFS standard representation:

```json
{ "/": "<CID>" }
```

Converting such properties of the payload to CBOR will record the CID in base256 (binary) representation as a byte string with tag 42.
If the property has a different structure or the string does not contain a valid CID, the JSON fragment is simply ignored for the purpose of link extraction.
Currently, no other JSON fragment leads to the emission of CBOR tags.

## Business background

We recommend that events be small but self-contained facts.
In may business cases, facts can include larger pieces of data — _blobs_ — like pictures taken in quality inspection steps.
Such blobs belong with the event but they are often not needed for computing Local Twin state or making reports.
Therefore we need to provide a way to store these data such that they are accessible when processing the event, but without burdening every event consumer with the bytes.

Handling blobs for our users has been pointed out as a valuable product feature in many conversations with prospective and current solution partners.
Allowing links to blobs to be recognised by Actyx is the basis for providing holistic support, like the managed synchronisation of blobs between devices based on need and storage capacity.

## Consequences

Application code is still free to emit any JSON payload it likes, the worst that can happen is that a (broken) link is recorded where none was intended.
This implies that

- Actyx shall not automatically resolve such links
- failure to replicate the linked blobs shall not be regarded as a fatal error
- access to the blob behind a link needs to be offered via a dedicated API to application code
