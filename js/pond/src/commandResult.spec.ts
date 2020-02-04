import { CommandResult } from '.'

describe('CommandResult.fold', () => {
  it('should handle undefined', () => {
    expect(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      CommandResult.fold(undefined as any)({
        sync: () => 1,
        async: () => 2,
        none: () => 3,
      }),
    ).toEqual(3)
  })
})
