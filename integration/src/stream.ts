/* eslint-disable prefer-const */
import http from 'http'
import { Event } from './event-service-types'

export const findEventsInStream = (
  options: http.RequestOptions,
  timeoutMs: number,
  findInPayloadValue: string,
  requestBody: string,
): Promise<boolean> =>
  new Promise((resolve, reject) => {
    let timeoutID: ReturnType<typeof setTimeout>

    const req = http.request(options, (res) => {
      expect(res.statusCode).toBe(200)

      timeoutID = setTimeout(() => {
        req.abort()
        clearTimeout(timeoutID)
        reject(`Cannot find value ${findInPayloadValue} in event stream`)
      }, timeoutMs)

      res.on('data', (d) => {
        const dataObject: Event = JSON.parse(d.toString())
        const hasFoundValue = dataObject.payload.value === findInPayloadValue
        if (hasFoundValue) {
          clearTimeout(timeoutID)
          req.abort()
          expect(dataObject.payload.value).toBe(findInPayloadValue)
          resolve(true)
        }
      })
    })

    req.on('error', (error) => {
      clearTimeout(timeoutID)
      reject(error)
    })

    req.write(requestBody)
    req.end()
  })

export const getEventsInStreamAfterMs = <T>(
  options: http.RequestOptions,
  timeoutMs: number,
  requestBody: string,
): Promise<T[]> =>
  new Promise((resolve, reject) => {
    let timeoutID: ReturnType<typeof setTimeout>
    let eventsFromStream: T[] = []

    const req = http.request(options, (res) => {
      expect(res.statusCode).toBe(200)

      timeoutID = setTimeout(() => {
        req.abort()
        clearTimeout(timeoutID)
        resolve(eventsFromStream)
      }, timeoutMs)

      res.on('data', (d) => {
        eventsFromStream.push(JSON.parse(d.toString()))
      })
    })

    req.on('error', (error) => {
      clearTimeout(timeoutID)
      reject(error)
    })

    req.write(requestBody)

    req.end()
  })
