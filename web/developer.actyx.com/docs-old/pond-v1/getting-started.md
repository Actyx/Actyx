---
title: Getting Started
hide_table_of_contents: true
---

Writing distributed apps is difficult. The **Actyx Pond** framework makes it simple.

The Actyx Pond is a highly opinionated framework for building always-available distributed apps on [ActyxOS](../os/general/introduction.md). It provides a programming model and system guarantees ideally suitable for use-cases requiring 100% availability. With this focus, the Actyx Pond can take care of concerns like eventual consistency, allowing you to concentrate on your business logic.

## Installation

The Actyx Pond is available on [npm](http://npmjs.com/package/@actyx/pond). To install in your project run:

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

## ActyxOS and Actyx Pond in 10 minutes

Here is a video that Alex, one of our engineers, made explaining Actyx in 10 minutes.

import YouTube from 'react-youtube';

<div className="embedded-yt-wrapper">
<YouTube
  videoId="T36Gsae9woo"
  className="embedded-yt-iframe"
  opts={{
    playerVars: { autoplay: 0 },
  }}
/>
</div>

## Problems?

Ask for help on [our GitHub repository](https://github.com/actyx/quickstart) or [Twitter](https://twitter.com/actyx) or email developer@actyx.io.

## Learn more

Read up on the Actyx Pond's innovative [Programming Model](programming-model.md) or jump to the different [_Guides_](guides/hello-world.md) to learn more about the different aspects of the Actyx Pond.
