---
title: Blob Storage
---

Storing, distributing and accessing **B**inary **L**arge **Ob**jects, aka. data blobs.

Apps often need to store and share data that does not really fit the event model. Examples may include photos, videos, large PDFs or machine learning models. In this section we will have a look at how ActyxOS allows you to do this.

## Basics

All sorts of data types may be represented as data blobs. Consider for example a quality inspection system that is capturing photos. These photos may be of relevance to other apps running on different devices. Perhaps, for example, so that humans can verify the decisions the quality algorithm is making.

## Storing and distributing blobs

Let's take the example of distributing the photos captured by our quality inspection system. We will assume the photos are stored somewhere on the edge device's filesystem whenever they are taken. Whenever a new photo is taken, we want to distribute that to the ActyxOS swarm.

In this example, we will assume we are building an app for the [Docker Runtime](/os/docs/docker-runtime.html) using Nodejs. Somewhere in our code, we will have the following logic:

```js
var fs = require('fs');
var request = require('request');

function onNewInspectionPhoto(filepath) {
    request.post({
        url: 'http://localhost:4455/api/v1/blobs',
        formData: { // Building a multipart/form-data request
            blob: fs.createReadStream(filePath),
            metadata: JSON.stringify({
                'semantics': 'com.myapp.quality-inspection-photos',
                'retentionPolicy': {
                    'policy': 'keepUntilTime',
                    'until': 28499383002
                }
            })
        }
    }, function(err, response, body) {
        if (err) throw err;
        if (response.statusCode !== 201) {
            console.log(`Error storing blob: ${body}.`);
            return;
        }
        var data = JSON.parse(body);
        var handle = data.handle;
        console.log(`Successfully uploaded. Got handle ${handle}`);
    });
}
```

Whenever a new photo is taken, you should&mdash;somewhere else in your code&mdash; call the `onNewInspectionPhoto` function and pass it the filepath to that new photo. That function will then perform a POST request to the local Blob Service to store the blob.

> Retention policies
>
> The Blob Service offers different retention policies for data blobs. Please refer to the [advanced guide](/os/docs/blob-service.html) for more information about supported retention policies.

After this request completes, this blob will automatically be available to other apps on other devices. ActyxOS automatically takes care of the peer-to-peer distribution. Let's have a look at what that means.

## Being notified about and accessing blobs

Imagine you are building the app that humans use to verify the work of the automated quality inspection system. You could do so by implementing a single-page application for the ActyxOS [WebView Runtime](/os/docs/webview-runtime.html).

For this app to work, it needs to (1) be informed about when a new photo has been uploaded, and (2) access the new photo to show it to the end-user. Let's see how that would work.

### Finding out that a new photo has been stored

Whenever a new blob is stored, the Blob Service automatically publishes an event on a special ActyxOS event stream that you can access. The event stream has the following properties:

- semantics: `com.actyx.os.blobs.metadata`
- name: `ActyxOS-BlobService-Metadata`

Finding out when a new photo has been published thus means subscribing to that event stream and seeing if the metadata matches the semantics. Let's see how that could work in the following example:

```html{28-32}
<html>
  <head>
    <script>
      (function() {
        window.addEventListener("DOMContentLoaded", event => {
          // We are using the Event Service endpoint here, not the Blob Service's
          fetch("http://localhost:4454/api/v1/events/subscribe", {
            method: "POST",
            body: JSON.stringify({
              subscriptions: [
                {
                  name: "ActyxOS-BlobService-Metadata",
                  semantics: "com.actyx.os.blobs.metadata"
                }
              ]
            }),
            headers: {
              "Content-Type": "application/json",
              Accept: "application/json"
            }
          })
            .then(r => r.body.getReader())
            .then(reader => {
              const dec = new TextDecoder();
              const loop = () => {
                reader.read().then(chunk => {
                  if (!chunk.done) {
                    var payload = JSON.parse(dec.decode(chunk.value));
                    if (payload.semantics === "com.myapp.quality-inspection-photos") {
                      console.log("A new blob has been published.");
                      showBlob(payload.handle);
                    }
                    loop();
                  }
                });
              };
              loop();
            });
        });
      })();
    </script>
  </head>
  <body>
    <img id="latestPhoto" />
  </body>
</html>
```

### Showing the new photo to the end-user

Now that we know that a new photo has been stored by the inspection system, we need to retrieve the blob and show it to the end-user. We will do this by implementing the `showBlob` function shown above.

```js{15-17}
// Helper function for transforming a buffer to Base64
function arrayBufferToBase64(buffer) {
  var binary = '';
  var bytes = [].slice.call(new Uint8Array(buffer));
  bytes.forEach((b) => binary += String.fromCharCode(b));
  return window.btoa(binary);
};

// This function is call from the code example above
function showBlob(handle) {
    // Get the image
    fetch(`http://localhost:4455/api/v1/blobs?${handle}`)
    .then(response => {
        response.arrayBuffer().then(buffer => {
            var photoAsB64 = arrayBufferToBase64(buffer);
            // Show the image
            var elem = document.getElementById('latestPhoto');
            elem.setAttribute('src', 'data:image/jpeg;base64,' + photoAsB64);
        })
    })
}
```

The result of this is that every time a new blob with the given semantics is stored&mdash;by any device in your swarm&mdash;that image will automatically be shown to your end-user.

## Learn more

- Read the [advanced guide](blob-service) for the Blob Service
- Refer to the Blob Service [API Reference](blob-api)

Or, jump to the next section and learn about how to build user interfaces with ActyxOS.
