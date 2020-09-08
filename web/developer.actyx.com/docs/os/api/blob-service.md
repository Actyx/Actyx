---
title: Blob Service
---

The **Blob Service** allows you to store and share arbitrary **B**inary **L**arge **Ob**jects.

:::warning Alpha version software
The Blob Service is currently in internal alpha. It is planned for **public release in 2020**. To stay up to date about upcoming releases please check out our [blog](https://www.actyx.com/news) where we post release notes and roadmap updates.
:::

With this document we aim to provide you a preview about how the Binary Blob service works as we work toward a beta release.

## Overview

Data plays a key role in programming the collaboration of humans and robots. Cameras may generate images valuable for an end-user, or designers may publish work instructions necessary for operators. The Blob Service provides you with the means to store, share and manage this type of data.

Key capabilities:

- Reliably store and disseminate arbitrary (non-event) data blobs
- Provide access to stored data from any edge device
- Automatically manage data throughout its lifecycle as instructed

:::note Data vs. events?
ActyxOS allows you to easily [create persistent event streams](../guides/event-streams.md). With proper encoding (e.g. [Base64](https://en.wikipedia.org/wiki/Base64)) you could theoretically store any type of data as event payloads. This is not, however, a good approach since events are meant to be small pieces of information that are rapidly ingested by consumers. Additionally, the [Event Service](event-service.md) does not allow you to delete events, meaning your distributed storage space may [quickly run out](../../faq/running-out-of-disk-space.md).
:::

## Basics

### Supported data types

You can store and share any type of data using the Blob Service, from simple JSON objects to large video files.

To give you an example, here are some examples of the type of data you could store using the Blob Service:

- Production plans in the Excel (`.xlsx`) file format
- Work instructions exported as PDF files
- High-definition videos showing how to maintain a machine
- Digital photographs captured by quality inspection cameras

### Data distribution

As shown in the following illustration, the Blob Service automatically distributes stored data between edge devices. As a developer, you do not have to worry about how that distribution occurs; you can simply retrieve the data from your app on any other edge device.

![data-distribution](/images/os/data-distribution.svg)
<!-- Source: https://docs.google.com/presentation/d/1mjL8V3tU8tFTboEOFrJg6gd6yXIHQBOnyDCoqico8hQ/edit#slide=id.g60c052b336_0_0 -->

:::info Distribution during network outages
ActyxOS uses the underlying network to transfer data. In the case of an outage or if the edge device is disconneted, the distribution is postponed until edge devices are reconnected.
:::

### Content addressability

The Blob Service does not have a concept of file- or, rather, blobnames. Instead, blobs are identified by their content. This is called [_content-addressable storage_](https://en.wikipedia.org/wiki/Content-addressable_storage). In ActyxOS, a data blob's name is derived by hashing its content.

Please refer to the usage example below for more detail about how this works in practice.

### Lifecycle and metadata

Once stored, data is automatically persisted and distributed throughout the ActyxOS swarm. In order to allow you or a system administrator to manage this stored data, metadata, i.e. data about the data, is used.

Each data blob stored using the Blob Service has the following metadata:

- Content hash
- Storage date
- Blob semantics (the meaning of the data)
- Blob size
- Retention policy
- Source device (the ID of the device that stored the data)
- Source app (the ID of the app that stored the data)

This metadata is used by ActyxOS for management throughout the data blob's lifecycle, especially automatic and manual blob deletion according to the retention policy. The metadata is also available to you&mdash;as a developer&mdash;through the Blob Service's built-in event stream, which produces events such as the one shown below whenever a data blob is stored:

```json
{
    "stream": {
        "semantics": "com.actyx.os.storage.metadata",
        "name": "ActyxOS-StorageService-Metadata",
        "source": "a263bad7"
    },
    "timestamp": 21323209392,
    "lamport": 39928,
    "offset": 192,
    "payload": {
        "handle": "ipfs:c58ca26c42ff04c0791f7f7dee1adb35/file1",
        "createdOn": 21323209387,
        "source": "a263bad7",
        "semantics": "com.actyx.d.manufacturing.quality.inspection.image",
        "bytes": 79800,
        "retentionPolicy": {
            "policy": "keepUntilTime",
            "until": 28499383002,
        },
        "sourceApp": "de.mycompany-inc.weldingrobotcontrolv1"
    }
}
```

## Usage

The Storage Service is one of the tools ActyxOS offers you to construct collaboration for humans and machines. You can store data, retrieve data, inform others about the storage and delete data as necessary.

Interaction with the Storage Service happens using a local HTTP API. The endpoint is `http://localhost:4455/api/v1/blobs`.

::: info Important
The API can only be accessed locally, i.e. at `localhost`. It is not meant to be accessed from other devices. The distribution of data blobs to and from other devices happens automatically in the background. You have access to all stored data blobs from your local device.
:::

### Storing data

You store data by performing a POST HTTP request with `multipart/form-data` content to the Blob Service endpoint. Consider, for example, a case where you wanted to store an image called `inspection-2393.jpg`. In order to store this blob, you need to POST both some metadata and the file itself.

Using cURL you could perform this request as follows:

```bash
# Assuming you are in the directory containing inspection-2393.jpg
$ curl -i -X POST -H "Content-Type: multipart/form-data" \
    -F "blob=@inspection-2393.jpg" \
    -F "metadata={
            \"semantics\":\"com.actyx.d.manufacturing.quality.inspection.image\",\"retentionPolicy\": {
                \"policy\": \"keepUntilTime\",
                \"until\":28499383002
            }};type=application/json" \
    http://localhost:4455/api/v1/blobs
# Response
> {
    "handle": "ipfs:c58ca26c42ff04c0791f7f7dee1adb35/file1"
}
```

:::note What about all the other metadata?
The Blob Service automatically adds all the other metadata such as Source ID, app ID, storage timepoint, etc.
:::

Notice that the Blob Service responded to the POST request. The Blob Service returns a JSON object containing the handle to the uploaded content. We will see in the next section how you would use this to retrieve the data blob at a later point of time.

### Retrieving data

You can retrieve stored blobs by submitting a GET request to the endpoint, with the blob’s handle as a query string.
Given the `ipfs:c58ca26c42ff04c0791f7f7dee1adb35/file1` handle, we could, for example, retrieve the data as follows:

```bash
$ curl -s -X "GET" \
    -H "Accept: application/mixed" \
    http://localhost:4455/api/v1/blobs?ipfs:c58ca26c42ff04c0791f7f7dee1adb35/file1`
```

:::note Automatic distribution
The retrieval example above would work on any device in your ActyxOS swarm since the Blob Service automatically distributes data to all swarm devices.
:::

### Retrieving metadata

You can also retrieve metadata about a data blob using the Blob Service's HTTP API.
This works by performing a GET request against the blobs endpoint with `metadata-` prepended to the handle.
Here is an example for retrieving the metadata of the `ipfs:c58ca26c42ff04c0791f7f7dee1adb35` blob:

```bash
$ curl -s -X "GET" \
    http://localhost:4455/api/v1/blobs?metadata-ipfs:c58ca26c42ff04c0791f7f7dee1adb35`
```

### Getting notified

The automatic distribution of data to other devices is one of the most powerful features of the Blob Service. Since it happens automatically, you must, however, be able to get notified when new data is distributed.

As shown above, the Blob Store enables this through the `ActyxOS-StorageService-Metadata` stream with `com.actyx.os.storage.metadata` semantics. Getting notified about new blob stores, thus means subscribing to this event stream using the [Event Service](/os/docs/events-service.html):

In a Node.js app you could, for instance, do this as follows:

```js{17-20}
const fetch = require('node-fetch')
fetch('http://localhost:4454/api/v1/events/subscribe', {
    method: 'POST',
    body: JSON.stringify({ subscriptions: [{
        semantics: 'com.actyx.os.storage.metadate',
        name: 'ActyxOS-StorageService-Metadata'
        }] }),
    headers: { 'Content-Type': 'application/json' },
})
    .then(response => response.body)
    .then(body => {
    body.on('data', chunk => {
        const event = JSON.parse(chunk.toString('utf-8'));
        const { payload } = event;
        const { semantics, handle } = payload;
        console.log('event:', JSON.parse(chunk.toString('utf-8')));
        if (semantics === 'com.actyx.d.manufacturing.quality.inspection.image' ) {
            console.log(`Interesting data blob has been uploaded to ${handle}`);
            // Then get the data blob using the `handle` and the Blob Service API
            // and do something useful with it.
        }
    })
```

:::note Event stream offsets
The example above is a simplified implementation. In order to not receive events you have already received in the past, you would keep track of an offset map and request only events that you have not yet received. Please refer to the [Event Service](/os/docs/event-service.html) guide for more information.
:::

### Deleting data

Data blobs cannot be manually deleted from within an app. Deletion happens automatically according to each data blob's individual retention policy. See the [storing data](#storing-data) section above for an example of how to set a policy.

:::note
The swarm administrator can manually delete any data blob at any time using the [Actyx CLI](../../cli.md).
:::

The Blob Service supports the following retention policies for data blobs:

#### `keepForever`

The data blob will never be deleted.

#### `keepUntilTime`

The data blob will be deleted after the given timestamp. The following properties must/can be used to further specify the policy's behavior.

| Property |          | Description |
|:---------|:---------|:------------|
| `until`  | required | The time (as a Unix epoch) until which the data blob must be kept |

#### `keepUntilNext`

The data blob will be deleted when the next data blob that meets all of the defined properties is stored.

| Property        |          | Description |
|:----------------|:---------|:------------|
| `semantics`     | optional | The semantics of the binary blob |
| `source`        | optional | The source of the binary blob (device) |
| `sourceApp`     | optional | The app that stored the binary blob |

:::note
If you set the `keepUntilNext` policy without setting any of the properties, your data blob will be deleted as soon as any other data blob is stored on any device in the ActyxOS swarm.
:::
