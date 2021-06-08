import { client, t } from 'netvar'
import { Pond } from '@actyx/pond'
import { RxPond } from '@actyx-contrib/rx-pond'
import { OrderFish, State as OrderState } from '../fish/orderFish'
import { MachineFish, State as MachineState } from '../fish/machineFish'
import { combineLatest, of } from 'rxjs'
import { switchMap, map } from 'rxjs/operators'
import { Fish } from '@actyx/pond'

// ----------------------------------------------------------------------
// |                                                                    |
// | Settings section                                                   |
// |                                                                    |
// ----------------------------------------------------------------------

// define default settings use during the local development process when
// when the app is not running in an ActyxOS runtime
const defaultSettings = {
  name: 'Wago Connector',
  plcIp: '192.168.0.99',
}
type Settings = typeof defaultSettings

// parse the JSON from the AX_APP_SETTINGS environment variable passed
// by the ActyxOS runtime or use default settings
const settings: Settings = JSON.parse(
  process.env['AX_APP_SETTINGS'] || JSON.stringify(defaultSettings),
)
const machineName = settings.name
const plcIp = settings.plcIp

// ----------------------------------------------------------------------
// |                                                                    |
// | global variables section                                           |
// |                                                                    |
// ----------------------------------------------------------------------

// define tags as shortcut
const machineTag = MachineFish.tags.machine.withId(machineName)
const stateTag = MachineFish.tags.state.withId(machineName)
const orderTags = (id: string) =>
  OrderFish.tags.order.withId(id).and(OrderFish.tags.orderForMachine.withId(machineName))

// global vars to represent the...
/** all to this machine assigned orders */
let availableOrders = [] as OrderState[]
/** the state of the local machine */
let machineState: MachineState = { stateType: 'undefined', name: machineName }
/** the current timer to simulate the work */
let currentTimer: NodeJS.Timeout | undefined = undefined

// ----------------------------------------------------------------------
// |                                                                    |
// | Helper section                                                     |
// |                                                                    |
// ----------------------------------------------------------------------

/**
 * helper function to observe all entities somehow listed in a registry
 * @param pond pond to observe the fish
 * @param registry registryFish that hold the information about the entities
 * @param mapFn Map the registryFish State to the properties of a entity fish
 * @param entityFn the factory to create the entity fish
 */
const observeAll = <RS, S, P, E>(
  pond: Pond,
  registry: Fish<RS, any>,
  mapFn: (state: RS) => P[],
  entityFn: (props: P) => Fish<S, E>,
) => {
  const rxPond = RxPond.from(pond)
  // observe registry fish
  return rxPond.observe(registry).pipe(
    // map the state of the registry fish to a list of properties to hydrate the entities
    map(mapFn),
    // switch the observable over to the entities and get all states together in on array (combineLatest)
    switchMap((s) =>
      // note: if there is no property the observe will emit an empty array.
      s.length === 0 ? of<S[]>([]) : combineLatest(s.map((name) => rxPond.observe(entityFn(name)))),
    ),
  )
}

// main entry point of the application. The RxPond (Pond with RxJs support) connects to ActyxOS
Pond.default().then(async (pond) => {
  console.log(`started ${machineName} plc ip: ${plcIp}`)

  // subscribe to the OrderRegistry and get all current active order assigned to this machine
  observeAll(pond, OrderFish.availableOrdersFor(machineName), Object.keys, OrderFish.of).subscribe(
    (orders) => {
      if (availableOrders.length !== orders.length) {
        // use the netvar list 3 to update the network variable in the PLC
        list3.setMore({
          // if there is an order set the orderAvailable to high
          orderAvailable: orders.length > 0,
          // if there are more orders available set the moreOrdersAvailable to high
          moreOrdersAvailable: orders.length > 1,
        })
      }
      // update the global var in this app to start the fist order on button click
      availableOrders = orders
    },
  )

  // when the machine is switched on, it posts an event that it is here but most likely disabled
  // as soon the PLC connection is established another state is emitted
  pond.emit(machineTag.and(stateTag), {
    eventType: 'setState',
    machine: machineName,
    state: 'disabled',
  })

  // observe the state of the machineFish representing this machine
  pond.observe(MachineFish.of(machineName), (state) => {
    // this log will appear in the node-manager or in the `ax logs tail ...` output
    console.log('machine state', state)
    // if the machine is active, a timer should be active to execute the order
    if (state.stateType === 'active') {
      const { duration, name } = state.order

      // emit an event that the order is started now
      pond.emit(orderTags(name), { eventType: 'started', machine: machineName, name })

      currentTimer = setTimeout(() => {
        // if the order is done, an event is emitted that the machine finished the order
        pond.emit(machineTag, {
          eventType: 'finished',
          order: state.order,
          machine: machineName,
        })
        // the same for the order.
        pond.emit(orderTags(name), { eventType: 'finished', name })
      }, duration * 1000)
    }

    // according to the state of the machine, the light on the demobox should be on or off
    if (state.stateType !== 'undefined') {
      const working = state.stateType === 'active'
      const done = state.stateType === 'finish'
      // turn on or off the workStarted light, regarding the machine is in the active state
      if (list3.get('workStarted') !== working) {
        list3.set('workStarted', working)
      }
      // turn on or off the workDone light, regarding the machine is in the finish state
      if (list3.get('workDone') !== done) {
        list3.set('workDone', done)
      }
    }
    // store the state of the machine
    machineState = state
  })

  /**
   * Eventhandle to react on changes on the plc.
   * see list1 definition
   *
   * @param key Variable that changed on the plc
   * @param value new value of this variable
   */
  const onStateChanged = (key: string, value: boolean) => {
    switch (key) {
      case 'emergency': {
        // if the emergency button is changed to 'PRESSED'
        if (value) {
          if (machineState.stateType === 'active') {
            // stop the current order
            currentTimer && clearTimeout(currentTimer)
            currentTimer = undefined

            // replace the order to make him fresh
            pond.emit(orderTags(machineState.order.name), {
              eventType: 'placed',
              ...machineState.order,
            })
          }
          // emit the event that the state changed to emergency
          pond.emit(machineTag.and(stateTag), {
            eventType: 'setState',
            state: 'emergency',
            machine: machineName,
          })
        } else {
          // if the emergency is released, the new state is emitted corresponding to the state
          // of the enable switch
          pond.emit(machineTag.and(stateTag), {
            eventType: 'setState',
            state: list1.get('enable') ? 'idle' : 'disabled',
            machine: machineName,
          })
        }
        return
      }
      case 'enable':
        // if the enable variable changed on the PLC,
        // emit a event that the machine state changed
        pond.emit(machineTag.and(stateTag), {
          eventType: 'setState',
          machine: machineName,
          // set the state corresponding to the new value
          state: value ? 'idle' : 'disabled',
        })
        // additionally, stop the order and set it back to idle
        if (!value && machineState.stateType === 'active') {
          currentTimer && clearTimeout(currentTimer)
          currentTimer = undefined

          // replace the order to make him fresh
          pond.emit(orderTags(machineState.order.name), {
            eventType: 'placed',
            ...machineState.order,
          })
        }
        return
      case 'working':
        // if the working button gets pressed
        if (value) {
          // if the machine is in the idle state and there is something to do, start a new order
          if (machineState.stateType === 'idle' && availableOrders.length > 0) {
            // get the first available order
            const [nextOrder] = availableOrders
            // validate it
            if (nextOrder && nextOrder.stateType !== 'undefined') {
              // emit a event that the machine state changed
              pond.emit(machineTag, {
                eventType: 'started',
                machine: machineName,
                order: nextOrder,
              })
            }
          }

          // if the machine is in the finish state, go back to the idle state
          if (machineState.stateType === 'finish') {
            pond.emit(machineTag, {
              eventType: 'setState',
              machine: machineName,
              state: 'idle',
            })
          }
        }
        return
    }
  }

  // connect to the PLC with the netvar (https://www.npmjs.com/package/netvar)
  // use the ip from the settings and the default post 1202
  const nvl = client(plcIp, 1202)

  // open the first list and add the eventhandler to it
  const list1 = nvl.openList(
    {
      listId: 1,
      onChange: onStateChanged,
    },
    // define the fields of the network variable list from the PLC
    {
      enable: t.boolean(0),
      active: t.boolean(1),
      emergency: t.boolean(2),
      working: t.boolean(3),
    },
  )
  // open the third network variable list from the PLC
  // this is a writable list. defined in the PLC
  // it is updated on change and every 2000 ms
  const list3 = nvl.openList(
    {
      listId: 3,
      cyclic: true,
      cycleInterval: 2000,
    },
    // define the fields of the network variable list from the PLC
    {
      orderAvailable: t.boolean(0),
      moreOrdersAvailable: t.boolean(1),
      workStarted: t.boolean(2),
      workDone: t.boolean(3),
    },
  )
})
