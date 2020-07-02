import { Observable } from 'rxjs'
import { Pond2 } from '../'
import { Fish, TagQuery } from '../pond-v2-types'

type Event = string
type State = ReadonlyArray<Event>

export const start = async () => {
  const pond = await Pond2.default()

  const aggregate: Fish<State, Event> = {
    subscriptions: TagQuery.union('t0', 't1'),

    initialState: [],

    onEvent: (state: State, event: Event) => [event, ...state],

    // CacheKey.namedAggregate('p-e-fish', 'my-process-id-100', 0)
    fishId: { name: 'test-entity' },
  }

  const cancel = pond.aggregate<State, Event>(aggregate, state =>
    console.log('updated state to', state),
  )

  const tags3 = ['t0', 't1', 't2']
  await pond
    .emitEvents({ tags: tags3, payload: 'hello' }, { tags: tags3, payload: 'world' })
    .toPromise()
  const q = pond.emitEvent(['t1'], 't1 only')
  q.subscribe(() => console.log('emission callback 0'))
  q.subscribe(() => console.log('emission callback 1'))
  await pond.emitEvent(['t2'], 't2 only').toPromise()

  await Observable.timer(500).toPromise()
  cancel()

  // should not be printed immediately
  await pond.emitEvent(['t0', 't1'], 'full match 2').toPromise()

  // The Promise behind `emitTagged` completing does not actually imply the store will be ready to serve the event already.
  await Observable.timer(500).toPromise()

  pond.aggregate<State, Event>(aggregate, state =>
    console.log('2nd start -- updated state to', state),
  )

  await pond.emitEvent(['t0', 't1'], 'full match 3').toPromise()
}

start()
