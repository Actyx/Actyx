# Bindings to Actyx for the Node Manager

Calling `npm run build` (or `npm run build -- --release`) builds the `node-manager-bindings.dll` and then copies the produced file to `./node-manager-bindings.node` to be used in Node.js as an addon.

Example:

```js
const bindings = require('./node-manager-bindings.node');
js.getNodeDetails(...)
```
