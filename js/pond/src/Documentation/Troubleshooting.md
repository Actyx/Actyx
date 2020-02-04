# Troubleshooting ipfs events

## Local

1) Check that you are running your daemon using the `--enable-pubsub-experiment` option to allow pubsub.

    This option will go away once pubsub is no longer considered experimental.

1) Check that you are sending on the right topic

    ```
    ipfs pubsub sub <my-topic> | jq
    ```

    You should see traffic after a few seconds.

    If you don't see traffic, figure out what topic you are *actually* sending on. In a browser app, check network tab
    in chrome. You should see requests like 

    ```
    http://localhost:5001/api/v0/pubsub/pub?arg=events-RKL-2&arg=%7B%22...
    ```

    Note that if you have enabled the `local` option (GET parameter in iris/goldberg and config option in headless pond), you
    won't publish to pubsub *at all*.
    
1) Check your local store

   a) Browser-based
   
       Open application/indexeddb in the chrome dev tools to see the index database
       
   b) Nodejs-based
   
       Open the sqlite store using e.g. `sqlite3 actyx-data/headless.sqlite`
       
   Note that if you run with the volatile option, you won't be able to inspect the index store, since it will be fully in memory.

## Remote

1) Check that you are connected to the swarm. `ipfs swarm peers` should return a few items.

    If it returns a huge list, **you might accidentally be connected to the public ipfs**.
    If it returns no peers, you are offline

2) Open a pubsub topic on both nodes and see if you can communicate

    ```
    node2 $> ipfs pubsub sub comm-test
    node1 $> ipfs pubsub pub comm-test 'hello from node1'
    ```
    
    You should immediately see the message from node1 at node2

    If you can't communicate via pubsub despite being in the same swarm, it might be that you are not directly connected, and
    intermediate nodes *do not subscribe* to the pubsub topic.
    
    `A --- B` works always
    `A --- B --- C` works only if nodes A, B and C are subscribed to the same topic. This is a limitation of floodsub which will go
    away soon, when gossipsub is merged.
    
    For now, just log into ipfs.actyx.net and do `ipfs pubsub sub <my-topic> > /dev/null`. This is also how we are going to solve
    this in the short term in a customer deployment.
    
3)  Check that you get the data

    Once you receive snapshots via pubsub from remote nodes, the only thing that remains to be done is to check if you can
    get the actual data.
    
    When receiving a snapshot like this
    
    ```javascript
    {"type":"snapshot","source":"onYrV8UcbQ2","roots":{"onYrV8UcbQ2":{"cid":"zdpuAzHej83zM3uM7znkCvk6ZGqJzp2KbuZspvniUWmGbrm4E","psn":23}}}
    ```
    
    Try running `ipfs dag get zdpuAzHej83zM3uM7znkCvk6ZGqJzp2KbuZspvniUWmGbrm4E/block | jq`
    
    You should see some events. If you don't, there is either something wrong with IPFS, *or* the sending device has gone offline
    after publishing the snapshot.
    
1) If all of this works, you should see events in ada.
