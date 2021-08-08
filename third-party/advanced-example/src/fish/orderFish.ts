import { Tag, Fish, FishId } from '@actyx/pond'
/**
 * Order Fish.
 * Very minimal integration to represent the state of an order with a given name or get a list of all open orders
 *
 * Events: PlacedEvent, StartedEvent, FinishedEvent
 * Tags: order
 * Fish: OrderFish.of('name'), OrderFish.registry
 */

// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// !
// ! all undocumented parts are documented in machineFish.ts
// !
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

// ----------------------------------------------------------------------
// |                                                                    |
// | State Section                                                      |
// |                                                                    |
// ----------------------------------------------------------------------

// [[start:states]]
export type UndefineState = {
  stateType: 'undefined'
}

/**
 * complete lifecycle of the order in one type definition with different stateTypes
 */
export type DefinedState = {
  stateType: 'idle' | 'active' | 'done'
  name: string
  duration: number
  machine: string
}
// [[end:states]]

export type State = UndefineState | DefinedState

export type RegState = Record<string, boolean>

// ----------------------------------------------------------------------
// |                                                                    |
// | Event Section                                                      |
// |                                                                    |
// ----------------------------------------------------------------------
// !
// ! see importend note about events in the machineFish.ts file
// !

// [[start:events]]
/**
 * Event when a new order is placed
 */
export type PlacedEvent = {
  eventType: 'placed'
  name: string
  duration: number
  machine: string
}
/**
 * Event, when a machine starts to work on this order
 */
export type StartedEvent = {
  eventType: 'started'
  name: string
  machine: string
}
/**
 * Event, when a machine finished the order
 */
export type FinishedEvent = {
  eventType: 'finished'
  name: string
}
// [[end:events]]
/**
 * union type All expected events the MachineFish will get from the store
 */
export type Event = PlacedEvent | StartedEvent | FinishedEvent

// ----------------------------------------------------------------------
// |                                                                    |
// | Tags Section                                                       |
// |                                                                    |
// ----------------------------------------------------------------------
const tags = {
  /**
   * All order events should be emit with this tag. with the order name `order.withId(name)`
   */
  order: Tag<Event>('order'),
  /**
   * tag for witch machine this order is generated
   */
  orderForMachine: Tag<Event>('order-for-machine'),
}

// ----------------------------------------------------------------------
// |                                                                    |
// | Fish Section                                                       |
// |                                                                    |
// ----------------------------------------------------------------------

/**
 * Function to reduce the order events to the map, describing the map of
 * existing orders
 *
 * Nearly the same as the MachineFish.registry
 * but this registry deletes the order as soon as a finish event occurs
 *
 * @param state current known order name
 * @param event ne incoming order event
 */
const registryOnEvent = (state: RegState, event: Event): RegState => {
  switch (event.eventType) {
    case 'placed':
      state[event.name] = true
      return state
    case 'finished':
      delete state[event.name]
      return state
    default:
      break
  }
  return state
}

/**
 * Define the OrderFish as a exported collection of factory functions and
 * the tags. This will provide you a cleaner interface on the import site.
 *
 * eg
 * ```typescript
 * import { OrderFish } from '../fish/orderFish'
 *
 * pond.observe(OrderFish.of('Order#1'), console.log)
 * pond.emit(OrderFish.tags.order.withId('Order#1'), examplePlaceEvent)
 * ```
 */
export const OrderFish = {
  /** defined tags from above */
  tags,
  /** factory to create a fish that represent one specific order name */
  of: (name: string): Fish<State, Event> => ({
    /** @see MachineFish */
    fishId: FishId.of('orderFish', name, 0),
    /** @see MachineFish */
    initialState: { stateType: 'undefined' },
    /** @see MachineFish */
    where: tags.order.withId(name),
    /**
     * The onEvent function reduces all incoming events to the state of
     * the order.
     *
     * in case of a placed event, the order is set or reset to the data
     * from the event, and uses the name from the factory to set the name
     *
     * start and stop is only valid if the state machine is valid to
     * transmission to the next stage. The old state transferred to the
     * new state, but the stateType gets updated.
     */
    // [[start:onevent]]
    onEvent: (state, event) => {
      switch (event.eventType) {
        case 'placed':
          return {
            stateType: 'idle',
            name,
            duration: event.duration,
            machine: event.machine,
          }
        case 'started':
          if (state.stateType === 'idle') {
            return {
              ...state,
              stateType: 'active',
            }
          }
          return state
        case 'finished':
          if (state.stateType === 'active') {
            return {
              ...state,
              stateType: 'done',
            }
          }
          return state

        default:
          break
      }
      return state
    },
    // [[end:onevent]]
    isReset: (event) => event.eventType === 'placed',
  }),
  /**
   * registry of all available orders
   * :note: Registry fish should only keep a list of the entities. They are more flexible to use
   */
  registry: {
    fishId: FishId.of('orderRegistry', 'reg', 0),
    initialState: {},
    where: tags.order,
    onEvent: registryOnEvent,
  } as Fish<RegState, Event>,
  /**
   * Copy of the registry fish but contains a filter for the machine
   *
   * a better solution would be
   */
  availableOrdersFor: (machineName: string): Fish<RegState, Event> => ({
    fishId: FishId.of('orderRegistryForMachine', 'reg', 0),
    initialState: {},
    where: tags.orderForMachine.withId(machineName),
    onEvent: registryOnEvent,
  }),
}
