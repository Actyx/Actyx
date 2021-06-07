---
title: Optimizing memory consumption of differential dataflow applications
author: Dr. Jan Pustelnik
author_title: Software Engineer at Actyx
author_url: https://github.com/gosubpl
author_image_url: /images/blog/jan-pustelnik.jpg
tags: [database, dashboards, reports]
---

In practical applications of [differential dataflow](https://docs.rs/differential-dataflow/) to analyze factory event streams
one frequently encounters a lot of strings. Those strings usually represent various
names - of inventory articles, workstations, or activities. However, the straightforward approach in data flows
rich in string objects may lead to a lot of unnecessary duplication which can result in high memory usage, unacceptable in small devices.
Let's take a look at how we have handled this problem in Actyx internal BI pipelines using the Actyx Rust SDK.

<!--truncate-->

## Introduction

Differential dataflow is written in [Rust](https://www.rust-lang.org/), a modern programming language that helps build reliable,
safe, and secure software.

:::note
See the [introduction to differential dataflow on developer.actyx.com] to get more context.
:::

[introduction to differential dataflow on developer.actyx.com]: https://developer.actyx.com/blog/2020/06/25/differential-dataflow/

One of the central ideas helping Rust achieve its goals is [ownership](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html).
Tracing the ownership of memory and variables helps avoid a large class of bugs that have caused security issues in the past
and continue to trouble users of other programming languages. However, when the compiler cannot make sure that the object's
ownership is seamlessly transferred, the programmer needs to use the `clone()` operation.
Because of ownership structuring of differential dataflow we need to clone a lot, which for strings means copying all the bytes.
This results often in very high memory usage, that can be a limiting factor, especially in
memory constrained environments like the Raspberry Pi or other IoT platforms.
Once looking into the memory usage statistics it is easy to see that most of the memory contents is taken by strings.
We can work around this problem as long as we remember that Rust's `.clone()` operation is more about ownership than about copying bits.

[introduction to differential dataflow on developer.actyx.com]: https://developer.actyx.com/blog/2020/06/25/differential-dataflow/

## Problem setting

Let's assume that we need to process events describing finished goods produced at the factory. Each
finished goods item has the following attributes: quantity of pieces produced (pcs),
article id (like `AGK75641`), human understandable article name like `Fork, Trifoil design line`,
workstation at which the good was reported (say `LATHE 3`) and order id (like `FG/1234567/2020`).
Let's model this as a Rust struct:

```rust
pub struct FinishedGoods {
    pub article_id: String,
    pub article_name: String,
    pub workstation: String,
    pub order_id: String,
    pub pcs: i64,
}
```

## ActyxOS approach

When working with ActyxOS, finished goods will probably come as a `payload` in an encompassing
`Event` data structure, which will look like that:

```rust
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
pub struct FinishedGoodsEvent {
    lamport: LamportTimestamp, // place first to sort in ascending (causal) order
    payload: FinishedGoods,
    timestamp: TimeStamp,
}
```

You might have noticed that for `FinishedGoodsEvent` struct we derive some interesting attributes: `Eq`, `PartialEq`, `Ord`, `PartialOrd` and `Abomonation`. These
are here for a reason - `Eq`, `PartialEq`, `Ord` and `PartialOrd` are needed because differential dataflow orders (sorts) and deduplicates whatever data are flowing through
the pipelines.

Now on to the remaining attribute: `Abomonation`. This one is required by the internal serialization mechanism employed
by the differential dataflow, which is not [Serde](https://serde.rs/) as most of the Rust ecosystem uses, but [Abomonation](https://github.com/TimelyDataflow/abomonation).
This is a very efficient binary wire encoding, resulting in good performance of the program but a bit cumbersome to define for complex data structures.

`Abomonation`, like `Serde` needs to be transitive, which means that if you want to build your struct out of parts, they also need to support
`Abomonation`, so the full definition of `FinishedGoods` would run like this:

```rust
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
pub struct FinishedGoods {
    pub article_id: String,
    pub article_name: String,
    pub workstation: String,
    pub order_id: String,
    pub pcs: i64,
}
```

Now we are ready to write a pipeline that will produce a summary of produced pieces, aggregated by `workstation` and `article_id`.

The result will be the following `ProductionSummary` record:

```rust
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
pub struct ProductionSummary {
    pub article_id: String,
    pub article_name: String,
    pub workstation: String,
    pub total_pcs: i64,
}
```

that will be propagated to the database.

We start with extracting the essential parts of the event containing the `FinishedGoods` record:

```rust
let extracts = events.map(|ev| FinishedGoodsEvent {
    lamport: ev.lamport,
    payload: ev.payload,
    timestamp: ev.timestamp,
});
```

and then run the processing pipeline, which groups `pcs` by `article_id` and `workstation` and calculates the `total_pcs`:

```rust
let out = extracts
    .group_by(|e| (e.payload.article_id.clone(), e.payload.workstation.clone())) // *
    .reduce(|(article_id, workstation), inputs, outputs| {
        // same article_id should mean same article_name
        let article_name = inputs
            .get(0)
            .map(|e| e.0.payload.article_name.clone())            // *
            .unwrap_or_default();
        let total_pcs: i64 = inputs
            .iter()
            // note we multiply by count below!
            .map(|(e, count)| *count as i64 * e.payload.pcs)
            .sum();
        outputs.push((
            ProductionSummary {
                article_id: article_id.clone(),                   // *
                article_name,
                workstation: workstation.clone(),                 // *
                total_pcs,
            },
            1,
        ));
    })
    .ungroup();
```

In the pipeline above we see a lot of `.clone()` operations applied to `String`s (lines indicated by a `*`). Given `article_id`
or `workstation` is cloned twice, which results in significant memory overhead, this pipeline could be troublesome to deploy
for larger volumes of data in memory-constrained environments.

Rust strings are modifiable, like in C++ and unlike their Java counterparts.
Furthermore, as C++ has departed from the copy-on-write (frequently abbreviated as CoW) approach for strings
due to the change in how processors are architected, similarly in Rust strings are not CoW by default.
That results in large memory usage occurring whenever we `.clone()` a string in Rust, because the contents
of the string get duplicated. However, as `.clone()` in Rust is about ownership semantics
more than actual copying of the information, this problem can be easily side-stepped.

## Optimizing memory usage

The initial instinct would be to use the [`std::borrow::Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html), which
is a copy-on-mutation smart pointer in Rust and wrap the string inside of it like this:

```rust
use std::borrow::Cow;
let cow_string = Cow::from("some_article_id");
// and then at the end of the pipeline
let s: String = cow_string.to_string();
// or
let s_ref: &str = cow_string.as_ref();
// to get String you could also use:
cow_string.into_owned();
// or
cow_string.as_ref().to_owned();
```

With this approach, however, we encounter two important issues. First, `std::borrow::Cow` does not have an `Abomonation` instance. Second, even
if we wrote one, the solution would not be optimal. Imagine two events for `FinishedGoods`, both having the same `article_id`. They would
be deserialized into two different strings, having the same content. Only after that we would avoid the duplication in the pipeline by
using the CoW approach.

Because the strings in the analytics pipelines usually are not mutated, the ideal approach would be to use [string interning](https://en.wikipedia.org/wiki/String_interning).
That would leave out only a problem of creating a suitable `Abomonation` instance. This path was selected in `ActyxOS` SDK - and is called
[ArcVal](https://docs.rs/actyxos_sdk/0.4.0/actyxos_sdk/types/struct.ArcVal.html).

The `ArcVal` essentially is an `Abomonation`-enabled container for holding references to immutable strings, with cheap clone operation and deduplication
of contained values (so if one creates two new `ArcVal<str>` instances with the same contents, memory will be allocated only once, unlike with `Refcell`
where allocation will be avoided only during clone operations). The Rust compiler enforces the immutability guarantee for us.

Using `ArcVal` requires importing it from the [`actyxos_sdk`](https://crates.io/crates/actyxos_sdk/0.4.0) crate:

```rust
use actyxos_sdk::types::ArcVal;
```

The data model definitions will look then as follows:

```rust
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Abomonation)]
pub struct FinishedGoods {
    pub article_id: ArcVal<str>,
    pub article_name: ArcVal<str>,
    pub workstation: ArcVal<str>,
    pub order_id: ArcVal<str>,
    pub pcs: i64,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Abomonation)]
pub struct ProductionSummary {
    pub article_id: ArcVal<str>,
    pub article_name: ArcVal<str>,
    pub workstation: ArcVal<str>,
    pub total_pcs: i64,
}
```

Note that the pipeline itself will not change at all! However, now all `.clone()` operations are essentially free! In our experience, string
interning in the pipelines reduces memory usage to less than 50% of original usage (frequently even better - to less than 30%, as would probably be the case for this pipeline).

## Summary

We have shown how to create a simple analytics pipeline with ActyxOS and optimize its memory usage using advanced features present in ActyxOS Rust SDK.
The whole code for the examples can be found in the [ActyxOS Dataflow repository](https://github.com/Actyx/actyxos_data_flow) under `examples/finished-goods-1`
and `examples/finished-goods-2`.
