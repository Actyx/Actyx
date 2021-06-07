/* std pattern */
import * as React from 'react'
import * as ReactDOM from 'react-dom'
import { Pond } from '@actyx-contrib/react-pond'
import { App } from './App'

let error: unknown = undefined
const onError = (e: unknown) => {
  error = e
  setTimeout(() => location.reload(), 2500)
}

function Loading() {
  const [time, setTime] = React.useState(0)

  React.useEffect(() => {
    setInterval(() => setTime(time => time + 1), 1000)
  }, [])

  return (
    <div>
      {/* show the user that the connection attempt is ongoing */}
      <p>Connecting to Actyx … (since {time}sec)</p>
      {
        /* showing the error is essential for useful failure response */
        error ? <p>Error: {error}<br/>Is Actyx running?</p> : undefined
      }
    </div>
  )
}

// use ReactDOM to render the application
ReactDOM.render(
  <React.StrictMode>
    {/* Pond initializes the connection to ActyxOS and draws the children when the connection is established */}
    <Pond loadComponent={<Loading />} onError={onError}>
      {/* App that can use usePond, useFish, useRegistryFish, ... */}
      <App />
    </Pond>
  </React.StrictMode>,
  document.getElementById('root'),
)
