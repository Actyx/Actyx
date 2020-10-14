---
title: where
hide_table_of_contents: true
---

`where` is for selecting the events that will be passed to a Fish’s [onEvent](./on-event). Events in
the Pond are selected via _tags_ which are attached at emission time.

This emits an event with tags 'foo' and 'bar':  
`pond.emit(Tags('foo', 'bar'), { hello: 'world' })`

To retrieve this event in a Fish, we may write `where: Tag('foo')`.  
Alternatively: `where: Tag('bar')`.  
Using `where: Tags('foo', 'bar')` we will also select the event, but now it _must_ have both tags.

<!-- TODO: Links to detailed docs -->
