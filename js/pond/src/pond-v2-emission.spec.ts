/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Fish, Pond, TagQuery } from '.'

const stateAsPromise = (pond: Pond, tags: TagQuery) =>
  new Promise((resolve, _reject) => pond.observe(Fish.eventsAscending(tags), resolve))

describe('application of commands in the pond', () => {
  it('should execute every emission-callback', async () => {
    const pond = await Pond.test()

    const emit = pond.emit(['t0', 't1', 't2'], 'hello')

    let cbCalled = 0

    const cb = () => (cbCalled += 1)

    emit.subscribe(cb)
    emit.subscribe(cb)

    await emit.toPromise()

    expect(cbCalled).toEqual(2)

    const events = stateAsPromise(pond, TagQuery.matchAnyOf('t0'))

    // Assert we emitted only once, despite multiple subscriptions
    expect(events).resolves.toEqual(['hello'])

    await pond.dispose()
  })

  it('should execute every emission-callback even after emission has finished', async () => {
    const pond = await Pond.test()

    const emit = pond.emit(['t0', 't1', 't2'], 'hello')

    // Make sure it’s completed
    await emit.toPromise()

    // Callbacks added now should still fire:
    let cb0 = false
    let cb1 = false

    emit.subscribe(() => (cb0 = true))
    emit.subscribe(() => (cb1 = true))

    expect(cb0).toBeTruthy()
    expect(cb1).toBeTruthy()

    const events = stateAsPromise(pond, TagQuery.requireAll('t1'))

    // Assert we emitted only once, despite multiple subscriptions
    expect(events).resolves.toEqual(['hello'])

    await pond.dispose()
  })
})
