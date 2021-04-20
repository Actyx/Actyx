import axios, { AxiosError } from 'axios'
import { ErrorResponse } from '../../event-service-types'

describe('event service', () => {
  describe('access v1', () => {
    it('should return error when user try to run v1', async () => {
      await axios
        .get('http://localhost:4454/api/v1/events')
        .catch((error: AxiosError<ErrorResponse>) => expect(error).toMatchErrorNotFound())
    })
  })
})
