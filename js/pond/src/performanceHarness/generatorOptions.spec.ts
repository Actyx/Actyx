/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { GeneratorOptions } from './generatorOptions'

const mkArgv = (text: string): string[] => ['/exec-path', '/path/to/command', ...text.split(' ')]

describe('generatorOptions', () => {
  it('should allow specifying generation options', () => {
    const argv = mkArgv(
      '--duration P1D --period PDT10S --remote ws://localhost:5000 --start 2018-01-01T00:00:00Z',
    )
    expect(GeneratorOptions.parse(argv)).toEqual({
      clean: false,
      dbName: undefined,
      duration: 86400,
      mode: 'ipfs',
      period: 10,
      remote: 'ws://localhost:5000',
      start: 1514764800000,
    })
  })

  // NOTE: unfortunately the commander library holds everything internally in global state
  // and therefore parse can be effectively called only once in the life-time of the program
  // TODO: consider other command line options parsing library and replace commander
  // unfortunately alternatives (e.g. https://github.com/vvakame/commandpost )
  // have issues or do not look actively maintained
  //
  // it('should allow omitting remote', () => {
  //   const argv = mkArgv('--duration P1D --period PT10s --start 2018-01-01T00:00:00Z')
  //   expect(GeneratorOptions.parse(argv)).toEqual({
  //     start: 1514764800000,
  //     duration: 86400,
  //     period: 0,
  //   })
  // })
})
