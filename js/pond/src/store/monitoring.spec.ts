import { prittifyArgs } from './monitoring'

describe('monitoring|prittifyArgs', () => {
  it('dont break', () => {
    expect(prittifyArgs({}, [])).toEqual([])
    expect(prittifyArgs(1, [])).toEqual([])
    expect(prittifyArgs(undefined, [])).toEqual([])
  })
  it('keep defined args untouched', () => {
    const msg = '%j + %j = %j'
    const args = [{ nr: 2 }, { nr: 2 }, { nr: 4 }]
    expect(prittifyArgs(msg, args)).toEqual(args)
  })
  it('just stringify and compress last parameter', () => {
    const msg = '%j + %j = '
    const args = [{ nr: 2 }, { nr: 2 }, { nr: 4 }]
    const params = [{ nr: 2 }, { nr: 2 }, '{nr:4}']
    expect(prittifyArgs(msg, args)).toEqual(params)
  })
})
