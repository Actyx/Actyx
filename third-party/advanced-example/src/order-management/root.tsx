/* std pattern */
import * as React from 'react'
import * as ReactDOM from 'react-dom'
import { Pond } from '@actyx-contrib/react-pond'
import { App } from './App'

const onError = () => {
  setTimeout(() => location.reload(), 2500)
}

// use ReactDOM to render the application
ReactDOM.render(
  <React.StrictMode>
    {/* Pond initializes the connection to ActyxOS and draw the children when the connection is established */}
    <Pond
      loadComponent={<div>Connecting to Actyx...</div>}
      onError={onError}
      manifest={{
        appId: 'com.example.order-management',
        displayName: 'Order Management App',
        version: '2.0',
      }}
    >
      {/* App that can use usePond, useFish, useRegistryFish, ... */}
      <App />
    </Pond>
  </React.StrictMode>,
  document.getElementById('root'),
)
