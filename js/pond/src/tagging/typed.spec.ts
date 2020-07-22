import { Emit, Fish, FishId } from '../types'
import { Tag, Tags, Where } from './typed'

type T0 = {
  type: '0'
  t0: object
}

type T1 = {
  type: '1'
  t1: object
}

const tag0 = Tag<T0>('0')
const tag1 = Tag<T1>('1')

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
const tagB = Tag<B>('B')

// Tag that covers 3 types
const abcTag = Tag<A | B | C>('ABC')

// Satisfy TS (no unused var)
const ignoreUnusedVar = (_v: unknown) => undefined

describe('typed tag query system', () => {
  // '0' and '1' have no overlap, so only 'A' remains
  const q = tag0.and(tag1).or(tagA)

  it('should prevent omission of event types covered by the tags', () => {
    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const q1: Where<'hello??'> = q

    expect(q1.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['0', '1'] }, { type: 'intersection', tags: ['A'] }],
    })
  })

  it('should insist on types?', () => {
    // @ts-expect-error
    const q2: Where<'A' | 'more-types'> = q

    // Must use q2 to avoid TS error...
    ignoreUnusedVar(q2)
  })

  it('should preserve the common event type', () => {
    // Overlap is 'A'
    const w = tagA.and(abcTag)

    // Errors because we cannot omit 'A'
    // @ts-expect-error
    const w2: Where<never> = w

    expect(w2.raw()).toMatchObject({
      type: 'intersection',
      tags: ['A', 'ABC'],
      onlyLocalEvents: false,
    })
  })

  it('should preserve local information', () => {
    // Overlap is 'A'
    const w = tagA.local().and(abcTag)

    expect(w.raw()).toMatchObject({
      type: 'intersection',
      tags: ['A', 'ABC'],
      onlyLocalEvents: true,
    })
  })

  it('should union event types ', () => {
    const u = tagA.or(tagB)

    expect(u.raw()).toMatchObject({
      type: 'union',
      tags: [{ type: 'intersection', tags: ['A'] }, { type: 'intersection', tags: ['B'] }],
    })
  })

  it('should union event types (complex)', () => {
    // Surface now is 'A', 'B', and 'C'
    const u = tagA
      .local()
      .or(tagB.withId('some-id'))
      .or(abcTag)

    // Also covers 'C' now
    // @ts-expect-error
    const u2: Where<'A' | 'B'> = u

    expect(u.raw()).toMatchObject({
      type: 'union',
      tags: [
        { type: 'intersection', tags: ['A'], onlyLocalEvents: true },
        {
          type: 'intersection',
          tags: ['B', 'B:some-id'],
          onlyLocalEvents: false,
        },
        { type: 'intersection', tags: ['ABC'] },
      ],
    })
  })

  describe('with Fish declarations', () => {
    const fishArgs = {
      onEvent: (state: undefined, _payload: A | B) => state,
      initialState: undefined,
      fishId: FishId.of('f', 'a', 0),
    }

    it('should require fish to implement onEvent that can handle all incoming events', () => {
      const fishWrong: Fish<undefined, A | B> = {
        ...fishArgs,

        // Expect error for too large subscription set
        // @ts-expect-error
        where: abcTag,
      }

      ignoreUnusedVar(fishWrong)
    })

    it('fish should accept direct Where<unknown> indicating untyped tag query', () => {
      const fishRight1: Fish<undefined, A | B> = {
        ...fishArgs,

        // Automatically type-inferred to match Fish declaration
        where: Tags('some', 'plain', 'tags'),
      }

      const fishRight2: Fish<undefined, A | B> = {
        ...fishArgs,

        // Automatically type-inferred to match Fish declaration
        where: Tag('a-single-plain-tag'),
      }

      ignoreUnusedVar(fishRight1)
      ignoreUnusedVar(fishRight2)
    })

    it('should accept OR-concatentation of untyped queries with explicit cast', () => {
      const fishWrong: Fish<undefined, A | B> = {
        ...fishArgs,

        // Without cast, this will fail
        // @ts-expect-error
        where: tagA.or(abcTag).or(Tag('foo')),
      }

      ignoreUnusedVar(fishWrong)

      const fishRight: Fish<undefined, A | B> = {
        ...fishArgs,

        // ... but adding an explicit cast solves the problem
        where: tagA.or(abcTag).or(Tag('foo')) as Where<A | B>,
      }

      ignoreUnusedVar(fishRight)
    })

    it('should accept additional untyped tags on an intersection', () => {
      const fishRight: Fish<undefined, A | B> = {
        ...fishArgs,

        // Type remains Tags<A>
        where: tagA.and('some-other-tag'),
      }

      ignoreUnusedVar(fishRight)
    })

    it('should allow fish to handle more events than indicated by tags', () => {
      const fishRight: Fish<undefined, A | B | C | T0> = {
        onEvent: (state: undefined, _payload: A | B | C | T0) => state,
        initialState: undefined,
        fishId: FishId.of('f', 'a', 0),

        // Fish declares it handles T0 also -> no problem.
        where: abcTag,
      }

      ignoreUnusedVar(fishRight)
    })
  })

  it('should allow emission statements into larger tags', () => {
    const emitRight = {
      payload: {
        type: 'A',
        data0: 5,
      },
      tags: abcTag,
    }

    return ignoreUnusedVar(emitRight as Emit<A>)
  })

  it('should forbid emission statements for unknown types, known tags', () => {
    const emitWrong: Emit<A> = {
      payload: {
        // @ts-expect-error
        type: 'whatever',
        data0: 5,
      },
      tags: tagA,
    }

    return ignoreUnusedVar(emitWrong)
  })

  it('should forbid emission statements into disconnected tags', () => {
    const payload: T0 = {
      type: '0',
      t0: {},
    }
    const emitWrong = {
      payload,
      tags: abcTag,
    }

    // @ts-expect-error
    return ignoreUnusedVar(emitWrong as Emit<T0>)
  })
})
