import fetch from 'node-fetch'
import { run } from '../../util'

describe('event service', () => {
  describe('access v1', () => {
    it('should return error when user try to run v1', () =>
      run((httpApi) =>
        fetch(httpApi + '/api/v1/events').then((x) => {
          expect(x.status).toEqual(404)
          expect(x.json()).resolves.toEqual({
            code: 'ERR_NOT_FOUND',
            message: 'The requested resource could not be found.',
          })
        }),
      ))

    it('should return proper error when endpoint does not exist', () =>
      run((httpApi) =>
        fetch(httpApi + '/api/v1/not-existing').then((x) => {
          expect(x.status).toEqual(404)
          expect(x.json()).resolves.toEqual({
            code: 'ERR_NOT_FOUND',
            message: 'The requested resource could not be found.',
          })
        }),
      ))
  })
})
