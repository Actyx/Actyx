/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Fish, FishId, Pond, Reduce, Tag, Tags, Where } from '.'

type PayloadWithTags<E> =
  | {
      tags: ReadonlyArray<string>
      payload: E
    }
  | undefined

const initialState = undefined
const onEvent: <E>() => Reduce<PayloadWithTags<E>, E> = () => (_state, payload, metadata) => ({
  tags: metadata.tags,
  payload,
})

const fishId = FishId.of('x', 'x', 0)

const stateAsPromise = <E>(pond: Pond, subs: Where<E>) => {
  const fish: Fish<PayloadWithTags<E>, E> = {
    where: subs,
    initialState,
    onEvent: onEvent<E>(),
    fishId,
  }

  return new Promise((resolve, _reject) => pond.observe(fish, resolve))
}

describe('application of commands in the pond', () => {
  it('should execute every emission-callback', async () => {
    const pond = Pond.test()

    const emit = pond.emit(Tags('t0', 't1', 't2'), 'hello')

    let cbCalled = 0

    const cb = () => (cbCalled += 1)

    emit.subscribe(cb)
    emit.subscribe(cb)

    await emit.toPromise()

    expect(cbCalled).toEqual(2)

    const events = stateAsPromise(pond, Tag('t0'))

    // Assert we emitted only once, despite multiple subscriptions
    expect(events).resolves.toEqual({ payload: 'hello', tags: ['t0', 't1', 't2'] })

    pond.dispose()
  })

  it('should execute every emission-callback even after emission has finished', async () => {
    const pond = Pond.test()

    const emit = pond.emit(Tags('t0', 't1', 't2'), 'hello')

    // Make sure it’s completed
    await emit.toPromise()

    // Callbacks added now should still fire:
    let cb0 = false
    let cb1 = false

    emit.subscribe(() => (cb0 = true))
    emit.subscribe(() => (cb1 = true))

    expect(cb0).toBeTruthy()
    expect(cb1).toBeTruthy()

    const events = stateAsPromise(pond, Tag('t1'))

    // Assert we emitted only once, despite multiple subscriptions
    expect(events).resolves.toEqual({ payload: 'hello', tags: ['t0', 't1', 't2'] })

    pond.dispose()
  })

  describe('with typed tags', () => {
    type A = { type: 'A'; data0?: number }
    type B = { type: 'B' }

    const tagA = Tag<A>('A')
    const tagAB = Tag<A | B>('AB')

    it('should attach all tags correctly', async () => {
      const pond = Pond.test()

      const tags = tagA.and(tagAB)
      const emit = pond.emit(tags, { type: 'A' })

      await emit.toPromise()

      const events = stateAsPromise(pond, tags)

      expect(events).resolves.toEqual({
        tags: ['A', 'AB'],
        payload: { type: 'A' },
      })

      pond.dispose()
    })

    it('should fail to compile if some tags cannot contain the event', async () => {
      const pond = Pond.test()

      const tags = tagA.and(tagAB)
      const payload: B = { type: 'B' }

      // tagA cannot contain events with type: 'B'
      // @ts-expect-error
      await pond.emit(tags, payload).toPromise()

      const events = stateAsPromise(pond, tags)

      expect(events).resolves.toEqual({
        tags: ['A', 'AB'],
        // Wrong but we made the compiler ignore it
        payload: { type: 'B' },
      })

      pond.dispose()
    })
  })

  describe('Pond emission type checking', () => {
    type T0 = {
      type: '0'
      t0: object
    }

    type A = {
      type: 'A'
      data0: number
    }

    type B = {
      type: 'B'
      data1: string
    }

    type C = {
      type: 'C'
      data1: number
    }

    const tagA = Tag<A>('A')

    // Tag that covers 3 types
    const abcTag = Tag<A | B | C>('ABC')

    const test = (_fn: (pond: Pond) => void) => {
      // Just make it easy to write declarations that use a Pond.
      // Since we only care about type-checks, we don’t actually execute anything.
    }

    it('should allow emission statements into larger tags', () => {
      test(pond =>
        pond.emit(abcTag, {
          type: 'A',
          data0: 5,
        }),
      )
    })

    it('should forbid emission statements for unknown types, known tags', () => {
      test(pond =>
        pond.emit(tagA, {
          // @ts-expect-error
          type: 'whatever',

          // actually it would pass if we used the `data0` field here,
          // due to some type-widening thingy
          dataFoo: 5,
        }),
      )
    })

    it('should forbid emission statements into disconnected tags', () => {
      const payload: T0 = {
        type: '0',
        t0: {},
      }

      // @ts-expect-error
      test(pond => pond.emit(abcTag, payload))
    })
  })
})
