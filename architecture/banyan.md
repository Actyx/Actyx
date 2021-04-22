# Banyan at Actyx

Banyan is a generic persistent sequence data structure with generic querying capabilities.

## Event sourcing

### Writing vs. reading

#### Writing

In a distributed event sourcing system, events are written at a relatively slow rate. E.g. events *per device* can be on average 1/min for human actions and 1/s for some automatic processes.

In cases where more data is being written, writing multiple events in a batch is always possible to increase efficiency. Alternatively, large events can be used.

#### Replication

Instant replication does not have to use the minimum number of bytes. It only has to be sufficiently efficient to be able to keep up with the rate at which events are being produced.

In addition, for a small update such as appending a single, simple event, there isn't much difference in the update size provided that it remains relatively small, below a kilobyte or so.

Bulk replication on the other hand should be as fast as possible, to heal partitions quickly. It should also not require much processing power for both participants.

#### Reading

Reading has to be very fast and selective. A typical event is written *once* on *one* device, but often read 100s or even 1000s of times from each device participating in a swarm (possibly 100s)

It is clear that a data structure for a distributed event sourcing system should be heavily optimized for reading. Due to the fact that data is read very frequently and sifted through even more frequently, it is acceptable to pay a heavy price when writing to make reading faster and more bandwidth efficient.

#### Summary

An event is written once (and then touched one or a few times more when packing the tree, or compacting the linked list in v1), replicated to other nodes ~100 times, read typically 100s of times on each of these nodes, and sifted through/skipped 100000s of times over its lifetime in the system.

Would be nice to have some real world numbers from CTA MWL here...

### Compression

ZStd compression has the awesome property that the compression rate can be tweaked over a wide range, from extremely fast compression with deflate/gzip compression ratios to slower compression with bzip2 level or better compression ratios. However, decompression is *always* fast, even for high compression factors. 

It follows from the above read/write tradeoff that zstd is a good choice for a distributed event sourcing system, whereas something that offers extremely fast compression such as lz4 would be a better choice for a more write optimized application.

### Encoding

We want an efficient, schemaless encoding to avoid coupling the storage format of the distributed event sourcing engine to application concerns. Encoding schemes that achieve this by packing the schema with the data have a disadvantage for small chunks of data, since the schema will then often be the majority of the data.

We chose CBOR as a binary and thus slightly more compact representation of JSON, that also fills some big gaps such as efficient representation of blobs, and is extensible via tags. One place where we already use tags is links to content-addressed data, via tag 42.

### Chunking

It is absolutely essential for performance and efficiency to have decent sized chunks of data to work with. Compression does not work well at all with small chunks, and the entire data pipeline from networking to encryption to disk io has some overheads that are *per operation*, not *per byte*.

In a well designed event sourcing application, events are often relatively small. It follows that multiple events have to be combined for efficient storage and bulk dissemination.

Therefore banyan makes sure that data that is going to be kept around for a long time is chunked in chunks that are sufficiently large.

There is plenty of prior art for chunking data this way. Among many others, the open source root data format from CERN, and the proprietary satmon archive format used at DLR for satellite and space station data.

As opposed to many other formats, banyan tries to achieve a constant size *compressed* block size, whereas formats such as root use a constant size uncompressed block size.

This requires some more complex bookkeeping, but has large advantages when dealing with a wide variety of data. Some data, such as links to content-addressed items or images, does not compress at all. Other data such as repetitive cbor objects compresses extremely well (typical compression factor 20-30).

So having fixed size uncompressed data blocks would lead to a factor 20-30 variation in actual data blocks, leading to large inefficiencies.

## Tree structure

### Querying

It is very clear that a tree data structure is needed for efficient sifting through the data.

Even if we already had a very efficient append only log data structure, such as hypercore or blake2 logs, we would have to flatten a tree structure to this append only log to allow for efficient querying. This is frequently done by the dat/hypercore community to allow indexed data access such as file system abstractions or geospatial indexes. The very early banyan commits also contained experiments in this direction.

Banyan is not just a tree because we are working with immutable content-addressed data, but *most importantly* for efficient querying.

### Additional considerations

A banyan tree should be able to be much larger than what fits in memory. It should allow for lazy loading.

Compared to typical in memory tree data structures, the penalty for following a link is *much* higher. So it is even more essential that the long term storage representation of a banyan tree is wide, not deep.

This is where the name banyan came from - it is not a very high (deep) tree but can grow extremely wide. It can also have multiple roots that over time become indistinguishable from the initial root.

An overarching design goal is to have frequently accessed data closer to the root than less frequently accessed data. This is useful for two reasons: first to reduce the size of data that has to be loaded into memory to sift through a tree, and second to allow for efficient in memory caching of *just* the frequently accessed data.

Taking this design goal seriously leads to some interesting and somewhat unusual results. Even an *individual* event consists of two parts: a part that is frequently accessed, and a part that is only accessed once it is determined that it is really needed. In banyan an event is split into key and value. Since keys are used to determine if values are needed in the first place, they are stored in the lowest branch level. Leafs are just sequences of values.

Despite being a single logical entity, an event is being stored at 2 levels in the tree. This is somewhat similar to data in a columnar data store, where a single row is spread over multiple columns.

Even without having additional indexes, this split provides large efficiency gains for querying when values are larger than keys.

The same separation can be done higher up in the tree. A branch node consists of (uncompressible) links as well as summaries of its children, and a summary of the entire branch node.


### References/Ideas:

OLAP vs OLTP

Appending via extend_unpacked is OLTP, distribution via the fast path is also OLTP, packing and distributing the packed representation is then the transition to am ore OLAP friendly format.

https://en.wikipedia.org/wiki/Online_analytical_processing

https://en.wikipedia.org/wiki/Online_transaction_processing