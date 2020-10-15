---
title: fishId
hide_table_of_contents: true
---

The `FishId` is the unique identifier of a Fish. It is used to implement caching, in order to
improve performance of Pond-based applications. Whenever a Fish is observed via `pond.observe` or
`pond.run`, and it has already been observed previously, we can avoid aggregating all its events
again: Instead we just build on the previous state.

## Constructing a good FishId

Because the `fishId` is used for caching, it’s important that if two Fish have the same `fishId`,
they are really the same fish. We have split the id intro three parts, to make it easy to conform to
this requirement:

### `entityType`

This should be a string describing the general thing modelled by your Fish. It can be thought of as
its namespace. In the apps we have written at Actyx, we used strings like "edge.ax.sf.User". But
one could also be less formal and just take "User" as entityType.
  
### `name`

The concrete thing the Fish identifies. For example the username. Sometimes the name is just
"singleton", if there exists just one Fish of the sort.
  
### `version`

A counter for logic changes in the Fish. Caching for Fish is persistent on disk. So when the
_program code_ of a Fish is changed, it must also be considered a different Fish. Since "entityType"
and "name" will still be the same, there is a plain version counter to identify code changes.
  
You should never lower the version number.
