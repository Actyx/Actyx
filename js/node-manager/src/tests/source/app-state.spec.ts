import { reducer } from '../../client/app-state'
import { AppActionKey, AppStateKey } from '../../client/app-state/types'

describe('app-state', () => {
  describe('reducer', () => {
    describe('reducer', () => {
      it('should show overview screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowOverview },
        )
        const expected = {
          key: 'Overview',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show setup user key screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowSetupUserKey },
        )
        const expected = {
          key: 'SetupUserKey',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show about screen', () => {
        const result = reducer(
          { key: AppStateKey.Overview, version: '' },
          { key: AppActionKey.ShowAbout },
        )
        const expected = {
          key: 'About',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show app signing screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowAppSigning },
        )
        const expected = {
          key: 'AppSigning',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show node auth screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowNodeAuth },
        )
        const expected = {
          key: 'NodeAuth',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show node diagnostics screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowDiagnostics },
        )
        const expected = {
          key: 'Diagnostics',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should show node details screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowNodeDetail, addr: 'localhost' },
        )
        const expected = {
          key: 'NodeDetail',
          version: '',
          addr: 'localhost',
        }
        expect(result).toEqual(expected)
      })

      it('should show generate swarm key screen', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.ShowGenerateSwarmKey },
        )
        const expected = {
          key: 'SwarmKey',
          version: '',
        }
        expect(result).toEqual(expected)
      })

      it('should set app version', () => {
        const result = reducer(
          { key: AppStateKey.About, version: '' },
          { key: AppActionKey.SetVersion, version: '2.0.0' },
        )
        const expected = {
          key: 'About',
          version: '2.0.0',
        }
        expect(result).toEqual(expected)
      })
    })
  })
})
