import { Tag, Fish, FishId } from '@actyx/pond'

/**
 * Machine Fish.
 * Very minimal integration to represent the state of a machine or get
 * a list of all existing machine names.
 *
 * Events: SetStateEvent, OrderStartedEvent, OrderFinishedEvent
 * Tags: machine, machine-state
 * Fish: MachineFish.of('name'), MachineFish.registry
 */

/**
 * Data type to define the content of a order
 */
export type Order = {
  /** Order name */
  name: string
  /** duration how long the order needs to be processed */
  duration: number
  /** machine to process this order */
  machine: string
}

// ----------------------------------------------------------------------
// |                                                                    |
// | State Section                                                      |
// |                                                                    |
// ----------------------------------------------------------------------

/**
 * default state to create the MachineFish
 *
 * This will be the initial state of the fish and he will iterate over the
 * incoming events. It is valide to create any fish you like. (OrderFish.of(uuid.v4()))
 * A Fish with no events will stay in this state and you get this state as a result.
 */
export type UndefinedState = {
  stateType: 'undefined'
  name: string
}

/**
 * If the emergency button is pressed the fish should be in this state
 */
export type EmergencyState = {
  stateType: 'emergency'
  name: string
}
/**
 * If the machine is switched off the fish should be in this state
 */
export type DisabledState = {
  stateType: 'disabled'
  name: string
}
/**
 * If the machine is switched on the fish should be in this state
 */
export type IdleState = {
  stateType: 'idle'
  name: string
}
/**
 * If the machine is working on an order the fish should be in this state
 * and keep the current order
 */
export type ActiveState = {
  stateType: 'active'
  name: string
  order: Order
}
/**
 * If the machine finished an order the fish should be in this state
 * and keep the completed order.
 */
export type FinishState = {
  stateType: 'finish'
  name: string
  order: Order
}
/**
 * union type with all Idle states
 */
export type IdleStates = EmergencyState | DisabledState | IdleState
/**
 * union type with states when the machine has an assigned order
 */
export type RunningState = ActiveState | FinishState

/**
 * union type with all states a machine could be in
 */
export type State = UndefinedState | IdleStates | RunningState

// ----------------------------------------------------------------------
// |                                                                    |
// | Event Section                                                      |
// |                                                                    |
// ----------------------------------------------------------------------
/**
 * ::: important :::
 *
 * The events should contain all information about what can happen.
 *
 * E.g.:
 * To publish an event that the state of a machine changed to idle, the
 * following event is emitted via ActyxOS:
 *
 * tag: ['machine' 'machine:press', 'machine-state']
 * event: {
 *   eventType: 'setState',
 *   machine: 'press',
 *   state: 'idle'
 * }
 *
 * Information in the tags should not be used to model the business logic.
 * You can find the machine name in the tags and in the event itself. The
 * reason for that is that the event should contain all information to
 * describe what happened, and the tags are only used to subscribe/filter the
 * event streams.
 */

/**
 * Event when the machine changed the state
 * state: new state: emergency, disabled, idle
 * machine: machine who changed the state
 */
export type SetStateEvent = {
  eventType: 'setState'
  machine: string
  state: IdleStates['stateType']
}

/**
 * Event when the machine started an order
 * machine: machine who started the order
 * order: order data, to know what order the machine is working on.
 *
 * If the order data changed before the machine started, but the machine
 * did not get the information due to a a network outtage, it would not
 * be possible to know the order data
 */
export type OrderStartedEvent = {
  eventType: 'started'
  machine: string
  order: Order
}
/**
 * Event when the machine finished an order
 * machine: machine who finished the order
 * order: order data to know what order the machine did
 *
 * please see OrderStartedEvent
 */
export type OrderFinishedEvent = {
  eventType: 'finished'
  machine: string
  order: Order
}

/**
 * union type All expected events the MachineFish will get from the Event Service
 */
export type Event = SetStateEvent | OrderStartedEvent | OrderFinishedEvent

// ----------------------------------------------------------------------
// |                                                                    |
// | Tags Section                                                       |
// |                                                                    |
// ----------------------------------------------------------------------

/**
 * Collection of tags, for easy reuse in the code.
 * This is not required and could be inlined, but it is highly recommended
 * to define the tags once and reuse them with the type definition
 */
const tags = {
  /**
   * All machineTages should get this tag (e.g.: tags.machine.withId('name'))
   */
  machine: Tag<Event>('machine'),
  /**
   * All setStateEvents need to have this additional tags
   */
  state: Tag<SetStateEvent>('machine-state'),
}

// ----------------------------------------------------------------------
// |                                                                    |
// | Fish Section                                                       |
// |                                                                    |
// ----------------------------------------------------------------------

/**
 * Define the MachineFish as a exported collection of factory functions and
 * the tags. This will provide you a cleaner interface on the import site.
 *
 * eg
 * ```typescript
 * import { MachineFish } from '../fish/machineFish'
 *
 * pond.observe(MachineFish.of('press'), console.log)
 * pond.emit(MachineFish.tags.machine.withId('press'), exampleStartEvent)
 * ```
 */
export const MachineFish = {
  /** defined tags from above */
  tags,
  /** factory to create a fish that represent one specific machine */
  of: (name: string): Fish<State, Event> => ({
    /**
     * fishId, required to reference the fish internally and do some
     * performance optimizations
     */
    fishId: FishId.of('machineFish', name, 0),
    /**
     * Initial state of the fish.
     * In this case, the machine fish starts with the undefined state
     * as described on top on this file
     */
    initialState: { stateType: 'undefined', name },
    /**
     * This fish will subscribe to all events with the tag
     * 'machine' and `machine:${name}` and will only receive the events
     * referenced to this machine
     */
    where: tags.machine.withId(name),
    /**
     * The onEvent function reduces all incoming events to the state of
     * the machine.
     *
     * In this case the setState will overwrite the machine state and
     * set it to the state in the event. There is no reason to validate
     * this event, because it happened in the real world and we can
     * not reject it.
     *
     * The started and the finished event is handled the same way. The
     * fact that the machine starts the order and finished the order cannot
     * be rejected. It happened.
     *
     * The finish case is a perfect example for the need to add the order
     * to the event.
     * If you use the `state.order` in this case, it could be wrong.
     */
    onEvent: (state, event) => {
      switch (event.eventType) {
        case 'setState':
          return {
            stateType: event.state,
            name,
          }
        case 'started':
          return {
            stateType: 'active',
            order: event.order,
            name,
          }
        case 'finished':
          return {
            stateType: 'finish',
            order: event.order,
            name,
          }
        default:
          break
      }
      return state
    },
  }),
  /**
   * In addition to the of factory, a registry is defined without a factory function.
   * The registry is going to keep a map with all existing machines as key.
   *
   * The map is much faster to update than check if the machine is already in a list
   * and just add it if it is not already there.
   */
  registry: {
    fishId: FishId.of('machineRegistryFish', 'reg', 0),
    /** Start with a empty map */
    initialState: {},
    /** Subscribe to the state change events */
    where: tags.state,
    /**
     * Add the machine key to the map.
     * See the other example in the order fish, how to remove it again
     */
    onEvent: (state, event) => {
      if (event.eventType === 'setState') {
        state[event.machine] = true
        return state
      }
      return state
    },
  } as Fish<Record<string, boolean>, SetStateEvent>, // define type for TypeScript type system
}
