---
title: From Events to ERP Bookings
author: Oliver Wangler
author_title: Software Engineer at Actyx
author_url: https://github.com/wngr
author_image_url: /images/blog/oliver-wangler.jpg
tags: [Actyx Pond, ERP, Integration]
---

The IT and OT landscape inside factories is extremely heterogeneous; many systems are necessary to efficiently and
effectively produce goods. This enables purpose-built software with fast iterations, but obviously requires integrations
between systems if data should be made available to other systems along the life cycle. In this article, we will develop
a one-way integration: exporting data originating from the warehouse to the ERP system.

<!-- truncate -->

In an earlier [blog post](/blog/2020/08/04/event-design-for-a-logistics-solution) we went through the process of designing event schemata for a warehouse logistics solution, and sketched the core functionality of the [`SkuFish`](/blog/2020/08/04/event-design-for-a-logistics-solution#event-design-and-aggregation-into-fish). This fish’s main responsibility is to track the meta data and current location of an individual stock keeping unit (SKU) inside a warehouse.

As the ERP system shall remain the record of truth for material, we now extend our solution with the functionality to
export material movements. In this post, we will implement an application designed to export bookings to an ERP
system.

## Implementation

We'll walk through the specifics of this implementation step by step in the following, starting from the API the ERP
system offers, converting the data originating from the shop floor, constructing an entity responsible for exporting,
and finally we will give an outlook on how to react to errors and failures.

### What the ERP expects

In this case, we're assuming that the ERP system offers a HTTP based API accepting material bookings, and ignore any
security considerations for the sake of this blog post.

Each movement done by the warehouse workforce can be exported 1-1 to the ERP; let's illustrate this with an example:

* SKU A (article no.. 42), Quantity 500 has been moved from location X to Z
* SKU B (article no.. 42), Quantity 1000 has been moved from location Z to Y

These two movements will result in quantity changes in locations X, Y, and Z for the article no. 42 in the state of the
ERP system. However, this logic is encapsulated in the interface provided by the ERP system, and requires exactly one
booking request to be made for each movement.

Let's take a look at the format the ERP expects for material movements:

```typescript
// Data model as expected by the ERP system
type ErpBooking = {
  barcode: string
  timestamp: Date
  article: string
  batch?: string
  quantity: number
  sourceWarehouse: number
  sourceLocation?: string
  targetWarehouse: number
  targetLocation?: string
  employee: string
  comment?: string
}

// This is for bookkeeping of exported bookings
type Booking = ErpBooking &
  // initially every booking is pending
  { state: { type: 'success' } | { type: 'error'; reason: string } | { type: 'pending' } }
```

We also add a convenience function to convert from the [`SkuFish`]'s `SkuMovedEvent` to the `Booking` type.

```typescript
// Convert from domain event to ERP data model
const toBooking = (movement: SkuMovedEvent, timestamp: Date): Booking => {
  const { sku, employee, from, comment } = movement
  const {
    barcode,
    articleNumber: article,
    quantity,
    warehouse: targetWarehouse,
    location: targetLocation,
    internalBatchNumber: batch,
  } = sku
  return {
    barcode,
    employee,
    comment,
    sourceWarehouse: from.warehouse,
    sourceLocation: from.location,
    article,
    quantity,
    targetWarehouse,
    targetLocation,
    state: { type: 'pending' },
    batch,
    timestamp,
  }
}
```

### MovementTrackingFish

Now we piece together the actual entity that is responsible for keeping track on which movements have been exported to
the ERP system, and their respective outcomes (they might be pending, in-flight, errored, or successful).

#### Events

The fish needs to consume `SkuMovedEvent`s to learn about new movements from the warehouse, and will emit either
`BookingSuccess` or `BookingErrored` events persisting the outcome of an actual booking:

```typescript
type BookingErrored = { type: 'movementErrored'; id: string; booking: Booking; reason: string }
type BookingSuccess = { type: 'movementSuccessful'; id: string; booking: Booking }
type EWrite = BookingErrored | BookingSuccess
type Event = SkuMovedEvent | EWrite
```

To keep track of pending requests, this fish keeps a log of bookings _to be booked_. After a booking has been executed, the
pending log is pruned. This logic can be formulated as:

```typescript
type State = {
  // requestId needs to be unique
  pending: { [requestId: string]: Booking }
}

const onEvent = (state: State, event: Event, metadata: Metadata) => {
  switch (event.type) {
    case 'skuMoved': {
      const booking = toBooking(event, metadata.timestampAsDate())
      // The eventId is guaranteed to be unique per ActyxOS swarm
      state.pending[metadata.eventId] = booking
      return state
    }
    case 'movementSuccessful':
    case 'movementErrored': {
      const { id } = event
      delete state.pending[id]
      return state
    }

    default: {
      return state
    }
  }
}
```

To get a predictable unique ID (remember: onEvent needs to be deterministic and pure) for each booking, we use the
`eventId` field of the event's meta data.

#### Tags

With the release of the Actyx Pond Version 2 (check out this [post] for an overview), an event can have any number of
tags, and can be queried using any combination of them. This means, an event is no longer bound to a single _event
stream_ originating from one fish, but can belong to many streams, and individually consumed. Here, instead of stringly
typed tags, we're using the `TypedTag` feature of the Actyx Pond to link event types to explicit tags.

[post]: /blog/2020/07/24/pond-v2-release

```typescript
const tags = {
  // Identifies event emitted by `SkuFish`
  skuMoved: Tag<SkuMovedEvent>('skuMoved'),
  erpBookingSuccess: Tag<BookingSuccess>('erpBookingSuccess'),
  erpBookingErrored: Tag<BookingErrored>('erpBookingErrored'),
  erpBooking: Tag<BookingSuccess | BookingErrored>('erpBooking'),
}
```

The `skuMoved` tag is used to identify the `SkuMovedEvent`, similarly, `erpBookingSuccess` and `erpBookingErrored`
identify the respective events of this entity. The `erpBooking` tag identifies any ERP bookings by their unique id, and
can be used to construct tags looking like `'erpBooking:ee2bba2e-ce45-4190-9563-8323f2c334f6'`.

Now that we have formulated all of the necessary bookkeeping, we can construct the complete `MovementTrackingFish`:

```typescript
const fish: Fish<State, Event> = {
  fishId: FishId.of('MovementTrackingFish', 'singleton', 0),
  initialState: { pending: {} },
  onEvent,
  where: tags.skuMoved.or(tags.erpBooking),
}

// Wrapper object, holding ..
export const MovementTrackingFish = {
  // the actual fish definition
  fish,
  // continuous state effect to export movements
  emissionController,
  // helper object holding available and relevant tags
  tags,
}
```

But wait, all this boilerplate, and we have not made a single request to the ERP system, yet! This is where the briefly
mentioned `emissionController` steps onto the stage.

#### Emissions to the ERP system

The `MovementTrackingFish` outlined above is fed by both external events, originating from the `SkuFish`, and its
internal events for bookkeeping. So far, we only implemented the conversion from `Movement`s to pending `Booking`s in
the `onEvent` function above. Now what's left is doing the actual API request, and persisting the outcome of the API
call:

```typescript
// Factory function to create a `StateEffect` for exporting material movements
const emissionController = (api: string): StateEffect<State, EWrite> => async (state, enqueue) => {
  // Iterate through the `MovementTrackingFish`'s pending log
  for (const [id, booking] of Object.entries(state.pending)) {
    const baseTag = tags.erpBooking.withId(id)
    try {
      // State effects can be async!
      const result = await fetch(api, { method: 'POST', body: JSON.stringify(booking) })
      if (result.ok) {
        enqueue({
          tags: baseTag.and(tags.erpBookingSuccess),
          payload: { type: 'movementSuccessful', id, booking },
        })
      } else {
        // API available, but something went wrong
        const { status, statusText } = result
        throw new Error(JSON.stringify({ status, statusText }))
      }
    } catch (error) {
      // This could also be retried with some backoff, but for this just fails
      enqueue({
        tags: baseTag.and(tags.erpBookingErrored),
        payload: {
          type: 'movementErrored',
          id,
          booking,
          reason: error.toString(),
        },
      })
    }
  }
}
```

This pipeline will be installed as a continuous state effect on the pond as follows:

```typescript
const pond = await Pond.default()

// Install continuous state effect
pond.keepRunning(MovementTrackingFish.fish, MovementTrackingFish.emissionController)
```

This implementation is straight forward, as it relies on the following facts:

* There is 1-1 to relationship between an event and an ERP booking
* When used together with `pond.keepRunning`, Actyx Pond guarantees that the installed state effect is executed in a
strictly serialized fashion, and all prior generated events are applied the state passed into every subsequent
execution, so no booking will be done multiple times

In a future article, we will look how to implement a n-to-1 relationship from events to bookings. In the meantime, you
can check out this blog post by our CTO Dr. Roland Kuhn on how to build a [reporting pipeline] using Differential
Dataflow.

[reporting pipeline]: /blog/2020/06/25/differential-dataflow

### Reacting to Failed Bookings

Now, as we saw above, the API requests to the ERP system could fail because of different reasons. In case of
unavailability, we might just implement a retry mechanism. In other cases, where certain business rules might prohibit
accepting a booking, this will need human intervention, usually by the warehouse logistics manager. For that, we may add
a user interface displaying a log of the last exported bookings and their error state. In a future post, we will explore
how to confidently extend an existing solution with such functionality, and deploy it as a new application running on top
of ActyxOS.
