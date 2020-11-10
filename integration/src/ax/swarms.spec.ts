import { runOnAll, runOnEach } from '../runner/hosts'
import { isCodeOk } from './util'
import { promises as fs } from 'fs'
import { Reponse_Swarms_Keygen } from './types'
import { pathExists } from 'fs-extra'

const FILE_PATH = 'temp-swarm-key'

const getKeyAndOutputFromResponse = (response: Reponse_Swarms_Keygen) =>
  response.code === 'OK'
    ? { swarmKey: response.result.swarmKey, outputPath: response.result.outputPath }
    : undefined

const isBase64 = (data: string) => Buffer.from(data, 'base64').toString('base64') === data
const isLen128 = (data: string) => data.length === 128
const isKeyValid = (key?: string) => key && isBase64(key) && isLen128(key)

describe('ax swarms', () => {
  describe('keygen', () => {
    test('must return status Ok for swarm state', async () => {
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Swarms.State())
      const areStatesValid = responses.every((r) => 'Ok' in r)
      expect(areStatesValid).toBe(true)
    })

    test('must return valid swarmKeys (128 length and base64)', async () => {
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Swarms.KeyGen())
      const areValidResponses = responses.every((r) => isCodeOk(r))
      expect(areValidResponses).toBe(true)

      const areKeyAndOutputValid = responses
        .map(getKeyAndOutputFromResponse)
        .every((k) => isKeyValid(k?.swarmKey) && Boolean(k))
      expect(areKeyAndOutputValid).toBe(true)
    })

    test('must return a unique valid swarmKeys', async () => {
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Swarms.KeyGen())
      const keys = responses
        .map(getKeyAndOutputFromResponse)
        .filter(Boolean)
        .map((x) => x?.swarmKey)
      const areKeysUnique = new Set(keys).size === keys.length
      expect(areKeysUnique).toBe(true)
    })

    test('must create a file with a valid swarmKey', async () => {
      const fileExists = await pathExists(FILE_PATH)
      if (fileExists) {
        await fs.unlink(FILE_PATH)
      }
      const [response] = await runOnEach([{}], false, (node) => node.ax.Swarms.KeyGen(FILE_PATH))
      const swarmKey = response.code === 'OK' && response.result.swarmKey
      const swarmKeyFile = await fs.readFile(FILE_PATH, 'utf-8')
      expect(isKeyValid(swarmKeyFile)).toBe(true)
      expect(swarmKey).toBe(swarmKeyFile)
      await fs.unlink(FILE_PATH)
    })

    test('must create a file with a unique valid swarmKey', async () => {
      const file1 = `${FILE_PATH}0`
      const file2 = `${FILE_PATH}1`
      const file1Exists = await pathExists(file1)
      const file2Exists = await pathExists(file2)
      if (file1Exists) {
        await fs.unlink(file1)
      }
      if (file2Exists) {
        await fs.unlink(file2)
      }
      await runOnAll([{}, {}], false, ([node1, node2]) =>
        Promise.all([node1.ax.Swarms.KeyGen(file1), node2.ax.Swarms.KeyGen(file2)]),
      )
      const key1 = await fs.readFile(file1, 'utf-8')
      const key2 = await fs.readFile(file2, 'utf-8')
      expect(key1).not.toBe(key2)
      await fs.unlink(file1)
      await fs.unlink(file2)
    })

    test('must return `ERR_INVALID_INPUT` when cannot write a swarm key to file since file key already exists', async () => {
      const fileExists = await pathExists(FILE_PATH)
      if (fileExists) {
        await fs.unlink(FILE_PATH)
      }
      await runOnEach([{}], false, (node) => node.ax.Swarms.KeyGen(FILE_PATH))
      const responses = await runOnEach([{}], false, (node) => node.ax.Swarms.KeyGen(FILE_PATH))
      responses.forEach((r) => expect(r).toMatchErrInvalidInput())
      await fs.unlink(FILE_PATH)
    })
  })
})
