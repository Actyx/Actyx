import { Actyx, Tag } from '..'

describe('testEventStore', () => {
  it('should allow setting the time', async () => {
    let t = 1000
    const timeInjector = () => t++
    const actyx = Actyx.test({ timeInjector })
    await actyx.publish([
      { tags: ['x'], event: 0 },
      { tags: ['x'], event: 1 },
    ])
    const events = await actyx.queryAllKnown({ query: Tag<number>('x') })
    expect(events.events.map((ev) => ev.meta.timestampMicros)).toEqual([1000, 1001])
  })
})
