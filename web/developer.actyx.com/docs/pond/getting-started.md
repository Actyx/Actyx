---
title: Getting Started
hide_table_of_contents: true
---

The Actyx Pond is a highly opinionated framework for building always-available distributed apps on [ActyxOS](../os/introduction.md). It provides a programming model and system guarantees ideally suitable for use-cases requiring 100% availability. With this focus, the Actyx Pond can take care of concerns like eventual consistency, allowing you to concentrate on your business logic.

## Installation

The Actyx Pond is available on [npmjs.org](http://npmjs.com/package/@actyx/pond). To install in your project run:

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<Tabs
  defaultValue="npm"
  values={[
    { label: 'npm', value: 'npm', },
    { label: 'yarn', value: 'yarn', },
  ]
}>
<TabItem value="npm">

```bash
npm install @actyx/pond
```

</TabItem>
<TabItem value="yarn">

```bash
yarn add @actyx/pond
```

</TabItem>
</Tabs>

To use the Actyx Pond features, then import as follows:

<Tabs
  defaultValue="ts"
  values={[
    { label: 'Typescript', value: 'ts', },
    { label: 'Javascript', value: 'js', },
  ]
}>
<TabItem value="ts">

```typescript
import { Pond } from '@actyx/pond'
```

</TabItem>
<TabItem value="js">

```javascript
var pond = require("@actyx/pond");
```

</TabItem>
</Tabs>

## Diving into the Pond

The best way to get started is by following the tutorial, starting with the obligatory [Hello World](guides/hello-world.md).
The tutorial teaches you the main concepts you will encounter when working with Actyx Pond in ten short lessons.
Just following this material should take no more than 1–2 hours, although you may well find yourself trying out extensions to the examples.
If you encounter questions or get stuck, don’t hesitate to ask us by [mail](mailto:developer@actyx.io) or via our [Discord channel](https://discord.gg/262yJhc).
You may also peruse or open issues on [the GitHub repository](https://github.com/actyx/quickstart).
