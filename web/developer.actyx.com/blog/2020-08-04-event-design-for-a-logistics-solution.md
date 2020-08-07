---
title: Event Design for Warehouse Logistics
author: Oliver Wangler
author_title: Software Engineer at Actyx
author_url: https://github.com/wngr
author_image_url: /images/blog/oliver-wangler.jpg
tags: [Actyx Pond, Event Design]
---

Coming from centralized application models, designing an application running on top of a
decentralized and distributed architecture requires a little mind shift. With help of a real world
example, this blog post introduces one of basic building blocks of architecting a
decentralized application: designing event schemata.

<!-- truncate -->

![image scanner app](/images/blog/event-design-for-a-logistics-solution/scanner-app.png)

The example is based on a real world production installation we developed for one of our customers
and that has been live for more than two years. This blog posts walks you through the basic approach to
model the target domain, in this case a warehouse logistics system, with abstractions offered by
the [Actyx Pond] based on event sourcing. We start off with some requirements gathering to learn
about the target domain, and the intended usage scenarios. Based on that, we derive
meaningful business events and aggregate these into clusters, which can be treated as a single unit.
External systems are then characterized and a clear system boundary of the event sourced system is
established. The post is rounded off with an outlook to future extensions to the system.

[Actyx Pond]: https://www.npmjs.com/package/@actyx/pond

:::note
ActyxOS provides you with the basic tools you need to build a decentralized event sourcing system.
For an introduction to the concept of event sourcing check out [this article].
[this article]: /docs/os/theoretical-foundation/event-sourcing
:::

# Domain and Requirements

Efficiently and effectively modeling and implementing business processes is a huge and ongoing
research field. One widely applied methodology to model complex software systems is Domain-driven
Design ([DDD]), which goes well together with event sourcing. We're going to loosely borrow some of
its concepts.

[DDD]: https://domainlanguage.com/ddd/

## The Goal

Envision yourself being the production manager of a contract packaging company,
for example filling glue into tubes: To achieve a high Overall Equipment
Efficiency (OEE) and delivery reliability, contract packaging companies require
precise intralogistics, warehouse management, and a tight integration into the
actual production. The existing ERP-provided material management solution our
customer had in use was lacking in several aspects: tracking of individual
stockkeeping units (SKUs), real-time feedback from the production line about
current demand, and support of adequate mobile scanners in several high-bay
warehouses with intermittent network connectivity.

## The Requirements

The core set of requirements the solution had to fulfill were:
- Support of ergonomic scanners for the warehouse workforce, to be used e.g. while driving forklifts.
    Precise identification of material in up to six meters of height. Logging must be possible in all
    locations within the different warehouses.
- Intuitive logging of material movements with support of quantity changes.
- Tracking of individual stock keeping units (SKUs): Internal and external batch number, quantity,
QA protocol, etc. allowing tracking and tracing across the whole lifecycle of produced goods.
- Transparency about movements and inventory.
- Source of truth for any material quantities and location should remain to be the ERP system,
    as there are many business rules installed about e.g. which material types are allowed in which
    parts of the warehouse. This effort is not to be duplicated, but should stay encapsulated in the
    ERP system.

In this blog post, we'll focus on a subset of the overall solution: 
* Movement of SKUs from and to different warehouse locations,
* tracking and identification of individual SKUs and the associated location.

Furthermore, it is given that the identification of SKUs and locations is done using [Data Matrix]
codes that are either attached visibly to the SKU or to the storage location. The encoded
identifier is a universally unique identifier for the respective entity. The creation of SKUs is out
of scope.

:::note
The process described in this post remains the same regardless of the overall system size.
Understanding the domain, identifying the individual bits and describing this in an ubiquitous
language, are the core activities in event design.
:::

[Data Matrix]: https://en.wikipedia.org/wiki/Data_Matrix

## Relevant Domain Events

Relevant events that occur in the business process:

* SKU moved: A single SKU has been moved from location A to location B.
* Quantity of an SKU has changed.
* SKU has been used up completely.

All of those events result from actions a user has taken physically within the warehouse, moving
goods and logging the movement on their handheld mobile scanner. For that, the user needs to identify
the originating location, the individual SKUs to be moved (incl. potential quantity changes), and
the target location. Meaning, a single command `moveSkus` leads to the generation of a combination
of the above-mentioned domain events.

# Event Design and Aggregation into Fish

Given the fact that all considered domain events are generated by a single command, it suggests
itself to model those grouped into a single _Aggregate_ (or _[Fish]_ in Actyx lingo), called the
`SkuFish`. There will be a single `SkuFish` for every SKU keeping track of exactly this SKU
identified by its unique identifier (which is encoded as a string in the Data Matrix code).
From here, we can start sketching out the actual event types based on our domain model:

```typescript
type SkuMovedEvent = {
  type: 'skuMoved'
  from: { warehouse: number; location?: string }
  to: { warehouse: number; location?: string }
  sku: SkuData
  employee: string
  comment?: string
}
```

It's good practice to include all necessary information for interpretation of
events within those events directly, as long as size permits (in this case that
would be the `sku` field). This simplifies downstream consumption
significantly.

```typescript
type QuantityChangedEvent = {
  type: 'skuQuantityChanged'
  from: number
  to: number
  sku: SkuData
  employee: string
  comment?: string
}
```

This event indicates a change of quantity for a SKU. In theory, this could also be done with the
`skuMoved` event presented above. But as this represents a meaningful business event in this use
case, it makes sense to have a dedicated event for it, which in itself is semantically meaningful.

```typescript
type SkuCreatedEvent = {
  type: 'created'
  sku: SkuData
}

export type Event =
  | SkuCreatedEvent
  | SkuMovedEvent
  | QuantityChangedEvent

export type SkuData = {
  sku: string
  place: { warehouse: number; location?: string }
  quantity: number
  supplierBatchNumber?: string
  internalBatchNumber?: string
  articleNumber: string
  incomingDate: Date
  bestBeforeDate: Date
}
```

For completeness, we also added the `SkuCreatedEvent`, whose origin is not discussed within this article. All events
are combined into a single exported type for later consumption. The `SkuData` type represents metadata for each SKU.
This metadata field `sku: SkuData` is added to every event, and represents the current state of the SKU as was known at
the source of the event.

The process we have followed here looks simple, but it is applicable to a broad range of cases: we start by observing
the facts that occur in the real world and distill them into self-contained information packages (the events), then we
group them by the entities they describe. With the event tagging mechanism it is not a problem if a single event is
equally relevant for two entities, just add the event to both by adding multiple tags.

With these types, the actual `SkuFish` could be defined as:
```typescript
type State = { type: 'unitialized' } | { type: 'sku'; sku: SkuData }

type SkuFish = Fish<State, Event>

const onEvent = (state: State, event: Event) => {
  const { sku } = event
  return { type: 'sku', sku }
}

// Fish factory function
const forId = (id: string): SkuFish => ({
  initialState: { type: 'uninitialized' },
  onEvent,
  where: Tags(`barcode:${id}`, 'sku'),
  fishId: FishId.of('SkuFish', id, 0),
})

export const SkuFish = {
  forId
}
```

Wiring this up to a UI, a potential command handler initiating a movement would look like:
```typescript
await pond
  .run(SkuFish.forId(id), () => {
    const payload: SkuMovedEvent = {
      type: 'skuMoved',
      employee,
      comment,
      sku,
      from,
    }
    const tags = Tags(`barcode:${id}`, 'sku')
    return [{ tags, payload }]
  }).toPromise()
```

[Fish]: /docs/pond/programming-model

# System Boundary

As the ERP system should remain the source of truth of all material movements, the events generated in the warehouse
need to be fed into the ERP system. Because of various reasons (compliance, quantity not sufficient, system unavailable
etc.) the ERP system might reject movements. In such cases, the state represented within the ERP system does not match
the real world: there is a clash between the facts from the shop-floor and the business rules modelled in the ERP
system. These clashes happened also with a paper-based system before the digitalisation and the resolution was obvious:
someone needs to undo the erroneous movements or correct the erroneous ERP data that prevented the movement from being
accepted. In keeping with the premises of digitalisation, such conflict resolution is also more widely accessible (to
anyone with a suitable device), more quickly communicated and thus more efficiently performed.

One important advantage of the approach shown above over a centralized solution where the scanners are integrated
directly with the ERP system is that the logistics workers are never held back by issues with or within the ERP system,
they can continue to work without impediment.

:::note
Check out how to keep a transactional system – such as the ERP system – in sync with an audit trail
of everything that has happened in the factory (the event log) in [this
post](/blog/2020/06/25/differential-dataflow).
:::

# Outlook

In future posts, we'll explore other parts of this solution. Amongst others, we'll add an additional
app for warehouse managers to be informed immediately about bookings and potential errors resulting
from them, how to do the actual bookings into the ERP system, and provide feedback to the warehouse
workforce.
