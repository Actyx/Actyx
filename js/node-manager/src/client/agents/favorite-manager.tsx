// Favorite Control
// ================

import { ContainerCell } from '../util/immutable-container'
import { Obs, Serv } from '../util/serv-react'
import { ObsValcon } from '../util/valcon'

export type FavoriteParams = ObsValcon<null | {
  initial: string[]
  set: (_: string[]) => void
}>

export const Favorite = (param: FavoriteParams) =>
  Serv.build()
    .channels((channels) => ({
      ...channels,
      initialized: Obs.make<void>(),
    }))
    .api(({ onDestroy, channels }) => {
      const data = {
        initialized: false,
        favorites: ContainerCell(new Set()) as ContainerCell<Set<string>>,
      }

      const attemptInitialize = () => {
        const initial = param.get()?.initial
        if (!data.initialized && !!initial) {
          data.initialized = true
          data.favorites.mutate((f) => {
            f.clear()
            initial.forEach((item) => f.add(item))
          })
          channels.initialized.emit()
          channels.change.emit()
        }
      }

      // The first time "initial" data from store is changed
      // set it to internal store
      onDestroy(param.obs.sub(() => attemptInitialize()))
      attemptInitialize()

      const persist = () => {
        if (!data.initialized) return
        param.get()?.set(Array.from(data.favorites.access()))
      }

      return {
        favorite: (addr: string) => {
          if (!data.initialized) return
          data.favorites.mutate((f) => f.add(addr))
          persist()
          channels.change.emit()
        },
        unfavorite: (addr: string) => {
          const favorites = data.favorites
          if (favorites === null) return
          data.favorites.mutate((f) => f.delete(addr))
          persist()
          channels.change.emit()
        },
        getFavorites: () => data.favorites?.access() || [],
        getFavoritesAsContainerCell: () => data.favorites,
      }
    })
    .finish()
