import * as React from 'react'
import { Typography, Button, Input, ToggleButtons, Toolbar } from '@actyx/industrial-ui'
import { useFish, usePond } from '@actyx-contrib/react-pond'
import { MachineFish } from '../fish/machineFish'
import { OrderFish } from '../fish/orderFish'

export const App = (): JSX.Element => {
  // Define some react states for the user interactions
  const [name, setName] = React.useState<string>('')
  const [duration, setDuration] = React.useState<number>(0)
  const [machine, setMachine] = React.useState<string>('')

  // Get the state of the MachineRegistry fish. We use this later to create a select field
  const machines = useFish(MachineFish.registry)
  // get the pond to emit events
  const pond = usePond()

  // click eventHandler to place the new order
  const placeOrder = () => {
    // check if the input is valid
    if (name === '' || duration === 0 || machine === '') {
      return
    }

    // prepare the tags for the event
    const orderTag = OrderFish.tags.order.withId(name)
    const orderForMachineTag = OrderFish.tags.orderForMachine.withId(machine)
    // emit the event with pond.emit
    pond.emit(orderTag.and(orderForMachineTag), {
      eventType: 'placed',
      duration,
      machine,
      name,
    })
    // reset the input field to avoid spamming
    setName('')
  }

  // create the react app.
  // I use the actyx industrial-ui to create shop-floor proven components
  return (
    <>
      <Toolbar variant="dark">
        <div style={{ width: '32px', marginRight: '24px', marginLeft: '24px', marginTop: '6px' }}>
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256">
            <path
              fill="#ffffff"
              d="M226.76,192.15V64.94a1.87,1.87,0,0,0-.94-1.63L116.6.25a1.91,1.91,0,0,0-1.88,0L7.38,62.22a1.89,1.89,0,0,0,0,3.27L60.11,95.93,114.72,64.4a1.86,1.86,0,0,1,1.88,0l53.67,31a1.89,1.89,0,0,1,.94,1.63v63.06h0l-29,16.74L116.6,191.6a1.86,1.86,0,0,1-1.88,0l-53.67-31a1.89,1.89,0,0,1-.94-1.63V95.93L5.5,127.5a1.87,1.87,0,0,0-.94,1.63v61.93a1.89,1.89,0,0,0,.94,1.63l109.22,63.06a1.91,1.91,0,0,0,1.88,0l54.61-31.53L196.83,239a1.88,1.88,0,0,0,1.89,0l51.78-29.9a1.88,1.88,0,0,0,0-3.26Z"
            />
          </svg>
        </div>
        <Typography variant="distance" color="#ffffff">
          Order Management
        </Typography>
      </Toolbar>{' '}
      <div
        style={{
          width: '100%',
          padding: '24px',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: 'white',
        }}
      >
        <div>
          <Typography variant="big">Place a new order</Typography>
        </div>
        <div style={{ marginBottom: '12px', marginTop: '24px' }}>
          <Typography variant="standard" bold>
            Order Number
          </Typography>
          <div style={{ display: 'flex', flexDirection: 'row' }}>
            <Input value={name} type="text" onChange={({ target }) => setName(target.value)} />
          </div>
        </div>
        <div style={{ marginBottom: '12px', marginTop: '24px' }}>
          <Typography variant="standard" bold>
            Planned Duration
          </Typography>
          <div style={{ display: 'flex', flexDirection: 'row' }}>
            <ToggleButtons
              items={[
                { id: '1', label: '1h' },
                { id: '3', label: '3h' },
                { id: '5', label: '5h' },
              ]}
              onToggle={(value) => setDuration(parseInt(value))}
            />
          </div>
        </div>

        <div style={{ marginBottom: '12px', marginTop: '24px' }}>
          <Typography variant="standard" bold>
            Machine
          </Typography>
          <div style={{ display: 'flex', flexDirection: 'row' }}>
            <ToggleButtons
              /*
               * the items of the machine select came from the machine registry.
               * I just map the keys of the registry state to the React data.
               *
               * As soon the state changes, the component is triggered automatically to redraw
               */
              items={Object.keys(machines.state).map((m) => ({ id: m, label: m }))}
              onToggle={(value) => setMachine(value)}
            />
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'row', marginTop: '24px' }}>
          <Button
            text="Place order"
            variant="raised"
            color="primary"
            // Add the click eventHandler to the button onClick
            onClick={placeOrder}
            disabled={name === '' || duration === 0 || machine === ''}
          />
        </div>
      </div>
    </>
  )
}
