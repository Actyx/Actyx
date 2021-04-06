---
title: React-Pond as simple as possible
author: Alexander Halemba
author_title: Software Engineer at Actyx
author_url: https://github.com/Alexander89
author_image_url: /images/blog/alexander-halemba.jpg
tags: [Actyx Pond, Setup, Project start, React-Pond, React, UI, Webview]
---

Are you looking for a framework to build your GUI?  
Are you starting to use React, or would you like to simplify your project?

**Then you definitely should take a look at the React-Pond.**

<!-- truncate -->

<img width="130px" src="https://raw.githubusercontent.com/actyx-contrib/react-pond/master/icon.png" />

## The problem

Up to now, in nearly every factory on this planet, you will find humans. Sooner or later, these people will want to interact in a successful shop-floor digitization project.

Even if you connect some machines, you will come to a point where you or your customer want to involve humans. It doesn't necessarily have to be a complex worker assistance application. I could be a maintenance assistant application or a small dashboard to see the machines' current state. Actyx provides you the **web-view runtime**, which can be installed via the [google play store](https://play.google.com/store/apps/details?id=com.actyx.os.android). As a developer, you can create any kind of **web-application** and package and deploy them to the edge-device.

As usual, the first step is the hardest. Where should you start? How to build a scalable and maintainable web-app with the pond and all your existing fish?

Don't panic, it's easier than you think.

## The solution

<!-- textlint-disable -->
The community in the area of web-applications is extremely large, and it makes sense to rely on existing frameworks like **React, Angular, Vue, jQuery, Ember, Backbone, ...** to name just a few.  
<!-- textlint-enable -->

Actyx is not bound to a specific framework. You can use Actyx with all of them, but in this blog post, I want to show you how easy it is to use React with the **React-Pond** integration made by the *community*.

## 🛠️ Setup a new React project with parcel

_If you already have an existing project, just cherry-pick the new Pond stuff._

Let's go into your folder of choice to set up a new React project based on TypeScript using Parcel as our build tool.  
_A basic requirement is that [Node.js](https://nodejs.org/en/download/) is installed on your PC_

### 1. Setup React and Parcel

```bash
npm init -y
npm install react react-dom
npm install -D parcel-bundler @types/react @types/react-dom typescript
```

### 2. Setup Actyx-Pond

As the next step, we have to install the `@actyx/pond` package, the required dependencies, and install the `@actyx-contrib/react-pond` package

```bash
npm install @actyx/pond fp-ts@1.19.4 io-ts@1.10.1 io-ts-types@0.4.1 rxjs@5.5.12
npm install @actyx-contrib/react-pond
```

### 3. `src` directory

To have a dedicated place for our source files, we create a new directory with the name `src` at the root of our project.

```bash
mkdir src
```

### 4. Configure TypeScript

Create a `tsconfig.json` file in the root of your project with the following content:

```json
{
  "compilerOptions": {
    "target": "es5",
    "module": "commonjs",
    "jsx": "react",
    "strict": true,
    "moduleResolution": "node",
    "lib": [ "dom", "es2017" ],
    "baseUrl": "./src",
  }
}
```

We need `"jsx"` specified as `"react"` and `"baseUrl"` for telling TypeScript where to look for our source files. Additionally, Parcel requires to set `"moduleResolution"` as `"node"`.

### 5. Summary: project setup

Your project should now look like that:

```plaintext
.
├── node_modules
├── package.json
├── package-lock.json
├── src
└── tsconfig.json
```

Now, we are ready to go, and we can start with our UI project

## 📑 Create a simple UI

### Entry point

First we have to create our entry point. Therefore we create a new file `index.html` in the `src` directory and add these lines:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Actyx-Pond-Demo!</title>
  </head>
  <body>
    <div id="root"></div>
    <script src="root.tsx" type="text/javascript"></script>
  </body>
</html>
```

### Initialize React and Actyx-Pond

Additionally we need the `root.tsx` file in the `src` directory. This is the TypeScript entry point defined in the HTML file above `<script src="root.tsx" [...]`. Here we can start to code our React application like this.

```typescript
// src/root.tsx
import * as React from 'react'
import * as ReactDOM from 'react-dom'
import { Pond } from '@actyx-contrib/react-pond'
import { App } from './App'

ReactDOM.render(
  <React.StrictMode>
    <Pond>
      <App />
    </Pond>
  </React.StrictMode>,
  document.getElementById('root')
)
```

- The `ReactDOM.render(..., document.getElementById('root'))` will render the HTML tags into a root div element made by React
- Best practice: `<React.StrictMode>` [React strict mode](https://reactjs.org/docs/strict-mode.html)
- Initialize your `<Pond>` for the entire project
- `<App />` will be our application what we will write

In this post, you will get a good overview of the features of the `<Pond>` tag, but if you'd like to dive deeper yet, you can check out the documentation [here](https://actyx-contrib.github.io/react-pond/globals.html#pond).

### Main App component

To have a practical example, we are going to write a small application for storage workers to fulfill material requests generated by a machine or a worker on the shop-floor. Of course, it's reduced in scope as much as possible.

Let's start with a very basic template with no variable data as a frame.  
This is the content of our `App.tsx` in the `src` directory:

```typescript
// src/App.tsx
import * as React from 'react'

export const App = () => {
  return (
    <div>
      <h1>Material Request: (id)</h1>
      <div>Status: (status)</div>
      <div>Material: (name) (amount)pc</div>
      <h3>
        <button onClick={() => alert('start')}>
          start
        </button>
        <button onClick={() => alert('finish')}>
          finish
        </button>
      </h3>
    </div>
  )
}
```

### 🚀 It's time to start the engine

Our project is now ready to start, and we use Parcel to compiling our app.

**Make sure that ActyxOS is *started* and *configured***  
checkout the [quickstart](https://developer.actyx.com/docs/learn-actyx/quickstart/#start-actyxos) and the [ActyxOS Node Manager](https://developer.actyx.com/docs/node-manager/overview) to set the settings of your ActyxOS Node if it is not already done.

Add two scripts to the `package.json` in the root directory of your project. The  `"scripts"` section should look like this on the end:

```json
  "scripts": {
    "dev": "parcel src/index.html --out-dir build/debug",
    "build": "parcel build src/index.html --out-dir build/release --public-url ./",
    "test": "echo \"Error: no test specified\" && exit 1"
  }
```

### The new scripts

- `npm run dev` Spin up the development server that automatically rebuild your project
- `npm run build` Build your project and put it into `./build/release`. Later you can package this web-application and deploy it to the tablet

Let's start the development server *(and ignore the warnings)*

```bash
npm run dev
```

You can open the web-app at [localhost:1234](http://localhost:1234)  
The output should look like this:
![image app start](/images/blog/react-pond/app-frame.png)
🎉🎉 Tada 🎉🎉

Cool, now we should add some distributed data and logic to it.

## 🐟 Create a fish

This blog post is **not about how to write a fish**. And I also assume that you already know how to do that. So, we just use the following example, which keeps the state of a material request and has the features to place a request, start it and finish it.

---
**Node:** I suggest that you use the VS Code plugin [Actyx-Pond](https://marketplace.visualstudio.com/items?itemName=Actyx.actyx-pond) to create fish for your projects. It creates all the type definitions for you and helps to have a common pattern in your projects.

---

Create a new file `src/materialRequestFish.ts` for the fish and copy the content into it.

```TypeScript
// src/materialRequestFish.ts
import { FishType, OnStateChange,
  Semantics, Subscription } from '@actyx/pond'

export type State =
  | { type: 'undefined' }
  | { type: 'idle' | 'started' | 'finished'
      material: string
      amount: number
    }

export type Event =
  | { type: 'placed', material: string, amount: number }
  | { type: 'started' }
  | { type: 'finished' }

export type Command =
  | { type: 'place', material: string, amount: number }
  | { type: 'started' }
  | { type: 'finished' }

const semantics = Semantics.of('com.example.MaterialRequest')
// MaterialRequestFish Definition
export const MaterialRequestFish =
  FishType.of<State, Command, Event, State>({
  semantics,
  initialState: name => ({
    state: { type: 'undefined' },
    subscriptions: [Subscription.of(semantics, name)]
  }),
  onEvent: (state, event) => {
    switch(event.payload.type) {
      case 'placed':
        const { material, amount } = event.payload
        return { type: 'idle', material, amount }
      case 'started':
        if (state.type !== 'undefined') {
          state.type = 'started'
        }
        return state
      case 'finished':
        if (state.type === 'started') {
          state.type = 'finished'
        }
        return state
    }
    return state
  },
  onCommand: (state, command) => {
    switch(command.type) {
      case 'place':
        const { material, amount } = command
        return [{ type: 'placed', material, amount }]
      case 'started':
        return state.type === 'idle' ? [{ type: 'started' }] : []
      case 'finished':
        return state.type === 'started' ? [{ type: 'finished' }] : []
    }
    return []
  },
  onStateChange: OnStateChange.publishPrivateState(),
})
```

## 🎣 Use the fish

Back to the UI. Now we want to show the state of the fish, representing a material request, on the screen.

The Pond integration is very similar to the *React hooks*.  
Instead of
  `const [value, setValue] = React.useState(initValue)`
you just write
  `const [fish, setFishName] = useFish(Fish, 'fishName')`

### Extend our UI

Let's open the `src/App.tsx` again and get the state of the fish

```typescript
// src/App.tsx
import * as React from 'react'
import { useFish } from '@actyx-contrib/react-pond' // add import
import { MaterialRequestFish } from './materialRequestFish' // import the fish

export const App = () => {
  const [matReqFish, setMatReqId] = useFish(MaterialRequestFish) // add useFish

// [...]
```

We don't set a fish name at the beginning, we are going to use the `setMatReqId` to select the material request with a `<select>` input and show the state of the material request if one is selected.

```typescript
// src/App.tsx
// [...]
  return (
    <div>
      {/* First we add a dropdown list to select the material request */}
      <select onChange={e => setMatReqId(e.target.value)}>
        <option></option>
        {/* In a later step, we replace this hardcoded
            values with a dynamic list */}
        <option>ID:0</option>
        <option>ID:1</option>
        <option>ID:2</option>
      </select>
      {/* We hide the material request UI, as long as no a ID is selected and fish  */}
      {matReqFish && (
        <>
          <h1>Material Request: ({matReqFish.name})</h1>
          <div>Status: {matReqFish.state.type}</div>
// [...]
```

The next part in our UI requires a check if the material request is defined and has a `material` and an `amount` set.

So, this
  ```<div>Material: (name) (amount)pc</div>```
turns into:

```typescript
// src/App.tsx
// [...]
          { matReqFish.state.type !== 'undefined' &&
            <div>
              Material: {matReqFish.state.material} {matReqFish.state.amount}pc
            </div>
          }
// [...]
```

And finally, we can feed the fish with commands when the user clicks the **start** or **finish** button.

```typescript
// src/App.tsx
// [...]
          <h3>
            <button onClick={() => matReqFish.feed({type: 'started'})}>
              start
            </button>
            <button onClick={() => matReqFish.feed({type: 'finished'})}>
              finish
            </button>
          </h3>
        </>
      )}
    </div>
  )
}
// [...]
```

YES, feeding a fish, it's that simple!

BUT wait, for this example, we have to simulate the `'placed'` event! There is no machine or anyone else who will request material.

## 🌊 Use the pond

There is more in the `@actyx-contrib/react-pond` than `<Pond>` and `useFish`. You also get `usePond`, `useRegistryFish`, `useRegistryFishMap`, and `useStream` in this package.

To simulate the `'placed'` events, we use `usePond()`. So we have a simple example that you still have all the functionality of the pond API.

Let's scroll up to the top of our `src/App.tsx` file and add `usePond` to the import.
  ```import { useFish } from '@actyx-contrib/react-pond'```
turns into
  ```import { useFish, usePond } from '@actyx-contrib/react-pond'```

Now we use the `usePond()` function in the App component, and feed the `MaterialRequestFish` with some random data.  
We add a new button on the top and if you click on that, a random `MaterialRequestFish` should be fed with a `place` command, containing the material and the amount.

```typescript
// src/App.tsx
// [...]
export const App = () => {
  const [matReqFish, setMatReqId] = useFish(MaterialRequestFish)
  const pond = usePond()

  return (
    <div>
      {/* button to place a random material request */}
      <button
        onClick={() => {
          const randomID = Math.round(Math.random() * 2)
          // feed the random MaterialRequestFish with the place command
          pond.feed(MaterialRequestFish, `ID:${randomID}`)({
            type: 'place',
            material: 'something',
            amount: Math.round(Math.random() * 100)
          }).toPromise()
        }}
      >
        Random material request
      </button>
      <br />
      {/* First we add a dropdown list to select the material request */}
// [...]
```

*If you like, you can create a second project to place material requests. But I recommend you to finish this blog post first.*

## 🎏 Use a registry fish in React

Lists, check boxes, or other data collection elements are essential for user interfaces. We will not dive too deeply into the registries ([registry fish pattern](https://developer.actyx.com/blog/2020/06/16/registry-fishes)), but we use the npm package `@actyx-contrib/registry` to create a registry fish for our material requests.

To install the registry package, run the install command in your project directory.

```bash
npm install @actyx-contrib/registry
```

As a next step we import `createRegistryFish` and `useRegistryFish` in the `src/App.tsx` file and create a registry fish. To get the state of all existing material requests, we just have to add the `useRegistryFish()` hook and we get an array with all materialRequestFish.

```typescript
// src/App.tsx
import * as React from 'react'
// add useRegistryFish
import { useFish, usePond, useRegistryFish } from '@actyx-contrib/react-pond'
// add this new import
import { createRegistryFish } from '@actyx-contrib/registry'
import { MaterialRequestFish } from './materialRequestFish'

const MaterialRequestRegistryFish = createRegistryFish(
  MaterialRequestFish, // entity to register
  'placed', // add fish on placed event,
  'finished', // remove fish on finished event
)

export const App = () => {
  const [matReqFish, setMatReqId] = useFish(MaterialRequestFish)
  const [materialRequests] = useRegistryFish(
    MaterialRequestRegistryFish, MaterialRequestFish
  )
// [...]
```

Finally we can use them to replace our hard coded select options like that:

```typescript
      <select onChange={e => setMatReqId(e.target.value)}>
        <option></option> {/*nothing selected*/}
        { // Map each material request to one option
          materialRequests.map(mrq =>
            <option key={mrq.name} value={mrq.name}>
              {/* we also can add the current state(type) now*/}
              {mrq.name} ({mrq.state.type})
            </option>
          )
        }
      </select>
```

Let's get wild in our place function and change

```typescript
  const randomID = Math.round(Math.random() *2)
```

into

```typescript
  const randomID = Math.round(Math.random()* 9999999)
```

## ♨️ RxJs observables/streams

The last thing I want to mention is the `useStream()` hook.

There are cases where you want to combine a couple of fish, do some mappings, add filters, or do other fancy things, and use the output in your application. It is even possible that you have an observable from somewhere else. 🤔

Every fish from `useFish()` has a property `stream$` and `useRegistryFish()` also offers you the stream of all fish states as second value in the returned tuple
  `const [array, states$] = useRegistryFish(RegistryFish, Fish)`

To make your life easier when working with the observables, you can use the `useStream()` hook in this way:

```typescript
  // Seconds since start
  const [tick] = useStream(Observable.interval(1000))

  // Pond getNodeConnectivity
  const { getNodeConnectivity } = usePond()
  const [nodeConnectivity] = useStream(getNodeConnectivity())

  // Fish state
  const [state, setOtherStream] = useStream(fish.stream$)
  // [...]
  setOtherStream(otherFish.stream$)
```

## Summary

![image app done](/images/blog/react-pond/app-done.png)

As always, a good toolchain is a foundation for great products.

The React-Pond package has been developed with the philosophy that it is simple, easy to learn, and similar to the well-known React hooks.

- `<Pond>`: Initialize the Actyx-Pond
- `usePond()`: Use the pond instance everywhere
- `useFish()`: Get the state, the name, an observable and the feed function of a fish
- `useRegistryFish()`: Map a registry fish to the entities and get a fish instance for each in an array
- `useRegistryFishMap()`: Same as useRegistryFish() but with a map function for advanced registry fish
- `useStream()`: Get the last value of an observable

Within this couple of minutes we created a small application that can run on multiple nodes, is partition tolerant, and has a persistent storage without touching any server, network, or data store. Isn't that amazing!?

The surface we have built together, however, is not yet suitable for the shop-floor environment. The requirements for component size and readability as well as the type of user interaction are often underestimated. We at Actyx have combined all our learnings in one NPM packet to save you from this headache. [Checkout the Actyx/industrial-ui](https://github.com/actyx/industrial-ui).

And of course, you can download the finished project [here (zip)](/images/blog/react-pond/react-pond-demo.zip)

## 📦 Community react-pond package

You can install the **react-pond** with `npm install @actyx-contrib/react-pond` in your project.

If you are hunting for more documentation, check out the README at the [repository](https://github.com/actyx-contrib/react-pond). You will also find some [more examples](https://github.com/actyx-contrib/react-pond/tree/master/example) in there, or you can read the full [API documentation](https://actyx-contrib.github.io/react-pond/).
