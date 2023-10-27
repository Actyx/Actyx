import { Destruction } from './destruction'
import { Obs } from './obs'

export { Obs }

type TDefaultAPI = {}
type TDefaultChannels = { change: Obs<void> }

type IdCont = {
  set: (id: string) => void
  lazyGet: () => Symbol
}

// eslint-disable-next-line @typescript-eslint/no-namespace
namespace IdCont {
  export const make = () => {
    let stringIdentifier = ''
    let cachedSymbol: Symbol | null = null
    return {
      set: (id: string) => {
        if (cachedSymbol !== null) {
          throw new Error('id() cannot be called when already used')
        }
        stringIdentifier = id
      },
      lazyGet: () => {
        const returned = cachedSymbol || Symbol(stringIdentifier)
        cachedSymbol = returned
        return returned
      },
    }
  }
}

export type Serv<API extends TDefaultAPI, Channels extends TDefaultChannels> = {
  id: Symbol
  channels: Channels
  api: API
} & Omit<Destruction, 'addHook'>

// eslint-disable-next-line @typescript-eslint/no-redeclare
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace Serv {
  export type DefaultAPI = TDefaultAPI
  export type DefaultChannels = TDefaultChannels

  type AgentBuilder<API extends DefaultAPI, Channels extends DefaultChannels> = {
    api: <NewAPI extends DefaultAPI>(
      fn: (input: AgentAPIDefinerParam<API, Channels>) => NewAPI,
    ) => AgentBuilder<NewAPI, Channels>
    channels: <NewChannels extends Channels>(
      fn: (channels: Channels) => NewChannels,
    ) => AgentBuilder<API, NewChannels>
    id: (id: string) => AgentBuilder<API, Channels>
    finish: () => Serv<API, Channels>
  }

  const makeBuilderImpl = <API extends DefaultAPI, Channels extends DefaultChannels>(
    prototype: AgentPrototype<API, Channels>,
  ): AgentBuilder<API, Channels> => {
    const finish = (): Serv<API, Channels> => ({
      id: prototype.idCont.lazyGet(),
      api: prototype.api,
      channels: prototype.channels,
      destroy: prototype.destruction.destroy,
      isDestroyed: prototype.destruction.isDestroyed,
    })

    const channels: AgentBuilder<API, Channels>['channels'] = (fn) =>
      makeBuilderImpl({
        ...prototype,
        channels: fn(prototype.channels),
      })

    const api: AgentBuilder<API, Channels>['api'] = (fn) =>
      makeBuilderImpl({
        ...prototype,
        api: fn({
          prev: prototype.api,
          channels: prototype.channels,
          onDestroy: prototype.destruction.addHook,
          isDestroyed: prototype.destruction.isDestroyed,
          id: prototype.idCont.lazyGet,
        }),
      })

    const id: AgentBuilder<API, Channels>['id'] = (id) => {
      prototype.idCont.set(id)
      return makeBuilderImpl(prototype)
    }

    return {
      id,
      finish,
      channels,
      api,
    }
  }

  export const build = () =>
    makeBuilderImpl<DefaultAPI, DefaultChannels>({
      api: {},
      channels: { change: Obs.make() },
      idCont: IdCont.make(),
      destruction: Destruction.make(),
    })
}

type AgentAPIDefinerParam<API extends TDefaultAPI, Channels extends TDefaultChannels> = {
  prev: API
  channels: Readonly<Channels>
  onDestroy: Destruction['addHook']
  isDestroyed: Destruction['isDestroyed']
  id: () => Symbol
}

type AgentPrototype<API extends TDefaultAPI, Channels extends TDefaultChannels> = Pick<
  Serv<API, Channels>,
  'channels' | 'api'
> & {
  destruction: Destruction
  idCont: IdCont
}

// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace ServUtils {
  export type GetterSetter<T> = {
    get: () => T
    set: (t: T) => void
  }
  // eslint-disable-next-line @typescript-eslint/no-namespace
  export namespace GetterSetter {
    export const make = <T, Channels extends TDefaultChannels>(
      { channels }: AgentAPIDefinerParam<any, Channels>,
      initVal: T,
    ): GetterSetter<T> => {
      let val = initVal

      return {
        get: () => val,
        set: (newval: T) => {
          val = newval
          channels.change.emit()
        },
      }
    }

    export const asApi =
      <T>(initVal: T) =>
      <Channels extends TDefaultChannels>(
        proto: AgentAPIDefinerParam<any, Channels>,
      ): GetterSetter<T> =>
        make(proto, initVal)
  }
}
