---
title: Query Events using the Node Manager
author: Oliver Stollmann
author_title: CEO and Co-Founder at Actyx
author_url: https://www.actyx.com/
tags: [Actyx Node Manager, Events, AQL]
---

Have you ever wanted to quickly find out what events have been created, and what they contain?
Or would you like to easily transform and copy a set of events?

With the new Query feature of [Actyx Node Manager 2.5.0](/releases/node-manager/2.5.0/) you can now do so easily. 

![node-manager-sample-query](../images/blog/node-manager-event-queries/sample-query.png) 

Read on to learn more.

<!-- truncate -->

## Query events using the Actyx Query Language (AQL)

AQL allows you to select, filter, and transform event streams. Here are a couple of examples.

### All events

```sql
FROM allEvents
```

![node-manager-query-from-all-events](../images/blog/node-manager-event-queries/from-all-events.png) 


### Filter by tags

```sql
FROM 'a' & 'a:2'
```

![node-manager-query-filter-by-tags](../images/blog/node-manager-event-queries/from-a-and-a2.png) 


### Transform payloads

```sql
FROM 'a' & 'a:2' SELECT _.key
```

![node-manager-query-transform-payloads](../images/blog/node-manager-event-queries/from-a-and-a2-select-key.png) 

### Learn more about AQL

To learn more about AQL check out the [AQL Reference](/docs/reference/aql)

## View event details and payload

You can click on an event to view all its details in an easy to navigate tree.

![node-manager-query-view-event](../images/blog/node-manager-event-queries/view-event.png) 

## Export events

If you want to perform further analysis of events, or just use some of them somewhere else, simply select the events you are interested it and copy them to your clipboard.
Then paste them — it's an array of JSON objects — anywhere else.

![node-manager-query-export-events](../images/blog/node-manager-event-queries/export-events.png) 

## Errors and warnings

If you query tries to access unknown properties, or performs illegal transformations (e.g. divide by zero), a warning or error message will be shown for each affected result:

![node-manager-query-errors-and-warnings](../images/blog/node-manager-event-queries/errors-and-warnings.png) 


## Questions or suggestions

We developed this feature based on user feedback. If you have questions or further suggestions, please don't hesitate to post in our [Community Forum](https://community.actyx.com/).
