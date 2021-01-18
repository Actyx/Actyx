---
title: Keeping your database up to date with Differential Dataflow
author: Dr. Roland Kuhn
author_title: CTO and co-founder at Actyx
author_url: https://rolandkuhn.com/
author_image_url: /images/blog/roland-kuhn.jpg
tags: [database, dashboards, reports]
---

Given an audit trail of everything that has happened in your factory, how do you keep the
dashboards, reports, and ERP system in sync with reality? This boils down to the problem of
maintaining materialized views of the event log. The state of the art is to update the
views — also in external systems — in a minimal fashion, changing only what needs to change and
exactly when the events happen.

<!--truncate-->

:::note
See also the [blog post on actyx.com] for a higher-level overview.
:::

[blog post on actyx.com]: https://www.actyx.com/news/2020/6/24/real-time_dashboards_and_reports_made_efficient_and_resilient

## The problem setting

In a factory, people and machines work together to produce goods, for example chairs. Multiple
intermediate products like legs need to be manufactured before the chair can be assembled, where
each piece is created and refined over a series of process steps at different workstations. Two
obvious questions for shop-floor personnel are: What is currently going on, and how much time has it
taken for the various people and machines to produce a given batch of chairs? These questions are
merely examples of the ones we could answer if we had an audit trail like this:

time | who | what
---|---|---
8:52 | Fred | starts setting up Drill1 for order 4711
8:56 | Fred | finishes setup
8:56 | Drill1 | starts working on order 4711
8:56 | Fred | starts drilling holes for order 4711
9:12 | Fred | reports 27 chair legs produced for order 4711
9:26 | Fred | reports 13 more chair legs produced for order 4711
9:27 | Fred | stops drilling holes for order 4711
9:27 | Drill1 | stops working on order 4711
9:27 | Fred | reports hole drilling for order 4711 is finished

Computers can help obtain such an audit trail, many of the entries can even be created automatically
or with very little additional input from a worker like Fred. One thing we need to ensure, though,
is that this reporting does not keep Fred nor the drill from performing their duty. The IT system
must be as reliable as paper — while being much easier to analyse later. This is why
[ActyxOS] uses a fully decentralised approach, recording
the events from the table above on the edge devices and synchronising between devices whenever a
network connection is available.

[ActyxOS]: http://developer.actyx.com/docs/os/general/introduction

## What we want to see

In this post we consider two simple cases out of the many that should be implemented at the factory
where Fred is working:

- a shop-floor dashboard shall show what is currently going on at Drill1 (analog for other
  workstations)
- a reporting database should be filled with summary information, one row per timespan that the
  machine was working for an order
- the ERP system should receive a booking for how much time Fred and Drill1 have spent working on
  this production step of order 4711 once Fred says that it is finished

The first part can be implemented using [Grafana](https://grafana.com/) if we keep a table in
[PostgreSQL](https://www.postgresql.org/) up to date with one row containing the information of what
is going on per workstation (e.g. which order is being processed and since when and by whom).

machine | doing what | since
---|---|---
Drill1 | working on order 4711 | 8:56
Drill2 | idle | 8:12
Drill3 | working on order 4712 | 8:33

The second part can be implemented similarly, by adding a row to a table of timespans whenever a
machine stops working on an order.

machine | order | started | duration
---|---|---|---
Drill1 | 4710 | 7:53 | 0:36
Drill2 | 4634 | 7:55 | 0:17
Drill1 | 4711 | 8:56 | 0:33

The third part works analog by creating an ERP transaction with the relevant bookings.

## How we want to program it

In order to get these processes right, it would be best to describe the required database changes or
ERP transactions based on patterns in the event log and then let a smart framework like
[Differential Dataflow] figure out how to run the corresponding computations as well as what changes
to commit when.  Much of this work can be done using filtering and aggregation, like using SQL on a
relational database.

[Differential Dataflow]: https://docs.rs/differential-dataflow

```rust
let (injector, events) = Flow::<Event<MachineEvent>, _>::new(scope);

let latest = events
    .filter(|ev| ev.stream.name.as_str().starts_with("Drill"))
    .map(|ev| match ev.payload {
        MachineEvent::Started { order } => {
            DashboardEntry::working(ev.stream.name.to_string(), order, ev.timestamp)
        }
        MachineEvent::Stopped { .. } => {
            DashboardEntry::idle(ev.stream.name.to_string(), ev.timestamp)
        }
    })
    .group_by(|entry| entry.machine.clone())
    .max_by(|entry| entry.since)
    .ungroup();
```

This code snippet operates on a [Flow] of events, which is a DSL built on the
[differential-dataflow](https://docs.rs/differential-dataflow) Rust library.

[Flow]: https://docs.rs/actyxos_data_flow/latest/actyxos_data_flow/flow/struct.Flow.html

- A flow is created within a scope of execution (see [the full example] for all details), returning
  an injector handle by which events can later be fed into this flow plus the `events` handle with
  which the data transformations are now described
- The `.filter()` method removes events which do not pertain to drills (just as an example), like
  `WHERE` in SQL,
- the `.map()` turns each event into a machine status dashboard entry, like `SELECT`

[the full example]: https://github.com/Actyx/actyxos_data_flow/tree/master/examples/machine-dashboard/logic.rs

The resulting `Flow` at this point describes a collection of status records, one for each relevant
event for each machine on the shop-floor.  Since we are only interested in the most recent status
update,

- we use `.group_by()` to split the collection into one group per machine and then
- take the maximum entry within each group sorted by timestamp

The `latest` variable now holds a description of a collection that contains one record per machine.
Whenever a new event is injected, this collection is updated accordingly. In the example, when the
event from Drill1 comes in at 8:56 that the machine has started working on order 4711 the `latest`
collection will have its previous record for workstation Drill1 removed and a new one inserted,
denoting that Drill1 has been working on order 4711 since 8:56.

## Going beyond SQL

One complication when working with an event log is that we need to correlate different
records (events) to compute for example the duration column in the reporting table. This can be done
in the dataflow DSL using the `.reduce()` combinator, which takes a function that turns a vector of
inputs into a vector of outputs. We do this separately for each machine (again using `.group_by()`)
to match each stop event with the corresponding start event for the same order, compute the duration
between them and emit a machine usage record.

```rust
let records = events
    .filter(|ev| ev.stream.name.as_str().starts_with("Drill"))
    .map(|ev| Excerpt {
        lamport: ev.lamport,
        machine: ev.stream.name.to_string(),
        event: ev.payload,
        timestamp: ev.timestamp,
    })
    .group_by(|excerpt| excerpt.machine.clone())
    .reduce(|_machine, inputs, outputs| {
        let mut started_events = BTreeMap::new();
        for (excerpt, _) in inputs {
            // inputs are in ascending order, so we know that stop comes after start
            match &excerpt.event {
                MachineEvent::Started { order } => {
                    started_events.insert(
                        order,
                        UsageEntry {
                            machine: excerpt.machine.clone(),
                            order: order.clone(),
                            started: excerpt.timestamp,
                            duration_micros: 0,
                        },
                    );
                }
                MachineEvent::Stopped { order } => {
                    if let Some(mut usage) = started_events.remove(&order) {
                        usage.duration_micros = excerpt.timestamp - usage.started;
                        outputs.push((usage, 1));
                    }
                }
            }
        }
    })
    .ungroup();
```

It is good practice to filter first and retain only the information necessary for later process
steps because the `.reduce()` operator needs to keep all inputs in memory: when a new event is added
for a machine, it will place it into its sorted slot in the `inputs` vector and run this function
again. While this sounds wasteful, it has two important advantages:

- it allows the dataflow framework to figure out what exactly changed in the outputs and only
  propagate that change further downstream (in our case: to the database)
- and it allows the code to be written with full focus on the business logic, without distractions
  for state management

:::info
We’ll get back to the question of long-running inputs below.
:::

To this end, it is often useful to introduce intermediate structures like `Excerpt` in the example
above. Besides giving useful names to its fields, it also provides the necessary sort ordering so
that the inputs are presented in causal order: we need to see effects after their cause, concretely
we need to see the machine stop after it has started. Sorting by normal timestamp does not always
achieve this, for example when the system clock of the edge device is modified or when there is
clock skew between edge devices. Therefore, ActyxOS tracks causality using so-called [Lamport
clocks]. The definition of the excerpt data structure is as follows:

[Lamport clocks]: https://en.wikipedia.org/wiki/Lamport_timestamp

```rust
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
struct Excerpt {
    lamport: LamportTimestamp, // place first to sort in ascending (causal) order
    machine: String,
    event: MachineEvent,
    timestamp: TimeStamp,
}
```

The derived trait instances are needed by the differential dataflow framework.

## Writing to the database

So far we have described only the business logic, now we want to write the results into an actual
database. First we need to spell out the details of such a record:

```rust
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
pub struct UsageEntry {
    pub machine: String,
    pub order: String,
    pub started: TimeStamp, // microseconds since the Unix epoch
    pub duration_micros: i64,
}
```

This record needs to be turned into one database row with four columns, to be inserted into a table
of some name. This is explained to the database driver by implementing the [DbRecord] trait.

[DbRecord]: https://docs.rs/actyxos_data_flow/latest/actyxos_data_flow/db/trait.DbRecord.html

```rust
impl DbRecord<SqliteDbMechanics> for UsageEntry {
    fn table_version() -> i32 {
        1
    }
    fn table_name() -> &'static str {
        "usage"
    }
    fn columns() -> &'static [actyxos_data_flow::db::DbColumn] {
        static X: &[DbColumn] = &[
            DbColumn {
                name: "machine",
                tpe: "text not null",
                exclude: false,
                index: true,
            },
            DbColumn {
                name: "manufacturing_order",
                tpe: "text",
                exclude: false,
                index: false,
            },
            DbColumn {
                name: "started",
                tpe: "timestamp with time zone",
                exclude: false,
                index: true,
            },
            DbColumn {
                name: "duration_micros",
                tpe: "bigint",
                exclude: false,
                index: false,
            },
        ];
        X
    }
    fn values(&self) -> Vec<<SqliteDbMechanics as DbMechanics>::SqlValue> {
        vec![
            Box::new(self.machine.clone()),
            Box::new(self.order.clone()),
            Box::new(self.started.as_i64() / 1_000_000),
            Box::new(self.duration_micros),
        ]
    }
}
```

This implementation is specific to the kind of database we want to write to because the column types
may depend on this information and the Rust data type for column values depends on the database
driver. In this case we’re targeting [Sqlite3] because it doesn’t require setup. You’ll want to
switch to [PostgreSQL] or [Microsoft SQL Server] for feeding your dashboards and reports.

[Sqlite3]: https://sqlite.org
[PostgreSQL]: https://postgresql.com
[Microsoft SQL Server]: https://www.microsoft.com/en-us/sql-server/sql-server-2019

## Transactional storage

We have described the contents of the database so far in terms of declarative business logic and a
table schema for our records. The final ingredient for knowing exactly which rows need to be in the
database is to denote the set of input events that have been processed so far. For this reason, the
database driver stores not only the records in the data table, it also stores — within the same
transaction — the [offset map] of the events that have been ingested into an adjacent table. In the
example above that table would be named `usage_offsets`.

[offset map]: https://docs.rs/actyxos_sdk/0.3.1/actyxos_sdk/event/struct.OffsetMap.html

This allows the process to be stopped and restarted without any loss of data: the business logic
will be (re)run on all events whose records are not yet in the database. Storing data and offsets in
the same transaction ensures that there can be neither duplicates nor losses.

Another benefit of this approach is that the restart of the exporter computes only the minimal
amount of events needed to become operational again, making this operation complete much quicker
than reprocessing all events would take.

## Reading the events

The final part is how to get the events from the [ActyxOS Event Service]. Since we want to read the
events in a Rust program, we first need to define their format and deserialization using [serde].

[ActyxOS Event Service]: http://developer.actyx.com/docs/os/api/event-service
[serde]: https://serde.rs

```rust
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Abomonation)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MachineEvent {
    Started { order: String },
    Stopped { order: String },
}
```

This matches the events emitted by the sample webview app, whose [Typescript] definition is

[Typescript]: https://www.typescriptlang.org/

```ts
export type Event =
  | { type: 'started'; order: string }
  | { type: 'stopped'; order: string }
```

With this, we have all the pieces to write the exporter’s `main()` function:

```rust
let mut db = SqliteDB::<Union<_>>::new("", "db_name")?;
let subscriptions = vec![Subscription::wildcard(semantics!("machineFish"))];

run_with_db_channel(
    runtime.handle().clone(), // Tokio runtime to use for running async tasks
    &mut db,                  // DB to store results in
    "dashboard",              // name for logging
    move |offsets, to_db| {
        run_event_machine_on_channel(
            Machine::new(&dashboard_logic),
            subscriptions, // which events we need
            offsets,       // where we left off last time
            to_db,         // sending channel towards DB
            "dashboard",   // name for logging
            1_000,         // events per transaction
        )
    },
)
```

Besides the helper functions that wire together the [EventService client], the differential dataflow
machine, and the database driver, the main part here is the definition of the event subscriptions. This
exporter needs to see all events emitted by the [machineFish]. The full code is available
[here](https://github.com/Actyx/actyxos_data_flow/tree/master/examples/machine-dashboard/main.rs).

[EventService client]: https://docs.rs/actyxos_sdk/latest/actyxos_sdk/event_service/struct.EventService.html
[machineFish]: https://github.com/Actyx/actyxos_data_flow/tree/master/webapp/src/machineFish.ts

## Avoiding endless growth

The setup described above should typically be sufficient for more than a year of data collection and
export. The limiting factor is the amount of state that needs to be stored within the dataflow
pipelines to generate the correct deltas. Due to the short-lived nature of processes on the factory
shop-floor, there is a natural solution to this problem: as e.g. manufacturing orders have a
lifetime of hours, days, or a few weeks, we can be pretty sure that events older than a year will
not have an influence on currently generated reporting data.

With this, we can provide a solution that avoids endless growth by restarting the exporter once per
year, initializing the internal state only from the events of the year before. This gives enough
context to the differential dataflow engine to emit the right deltas going forward. The
`actyxos_data_flow` library supports this with the [new_limited] function to construct input
collections:

[new_limited]: https://docs.rs/actyxos_data_flow/latest/actyxos_data_flow/flow/struct.Flow.html#method.new_limited

```rust
let one_year = Duration::from_secs(365 * 86400);
let (injector, events) = Flow::<Event<MachineEvent>, _>::new_limited(scope, one_year);
```

This also avoids reading the whole event history upon first start, it will only ingest the last year
of data — this can be very helpful in case the data collection has been ongoing in that factory for
a much longer time already.

## Summary

To summarize, [actyxos_data_flow] is a library that makes live data export from ActyxOS into
transactional databases easy, efficient, and resilient. The process can be restarted at any time
without data losses or duplications, and the programmer can concentrate fully on the business logic
and the table schema without having to worry about how to get the events or how to keep track of
what was already processed.

[actyxos_data_flow]: https://docs.rs/actyxos_data_flow
