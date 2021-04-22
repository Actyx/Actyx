import axios, { AxiosError } from 'axios'
import { ErrorResponse } from '../../event-service-types'

describe('event service', () => {
  describe('others', () => {
    it('should return proper error if endpoint does no exist', async () => {
      await axios
        .get('http://localhost:4454/api/v1/not-existing')
        .catch((error: AxiosError<ErrorResponse>) => expect(error).toMatchErrorNotFound())
    })
  })
})
