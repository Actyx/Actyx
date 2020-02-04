import * as fs from 'fs'
import * as pako from 'pako'
import { Timestamp } from '../'
import { DagCbor } from './dagCborUtil'

describe('dagCborUtil', () => {
  const { encode, decode } = DagCbor
  it('should serialize any json', async () => {
    // check that simple JSON is encoded
    const test = { a: 1, b: 'foo', c: [1, 2, 3], d: { nested: true } }
    const result = await encode(test).toPromise()
    expect(result.toString('hex')).toEqual('a4616101616263666f6f6163830102036164a1666e6573746564f5')

    const reverse = await decode(result).toPromise()
    expect(reverse).toEqual(test)
  })

  it('should serialize naked buffers', async () => {
    // check that naked buffers are encoded
    const buffer = Buffer.from('test')
    const result = await encode(buffer).toPromise()

    const reverse = await decode(result).toPromise()
    expect(reverse).toEqual(buffer)
  })

  it('should serialize nested buffers', async () => {
    // check that nested buffers are encoded
    const buffer = Buffer.from('test')
    const test = { theBuffer: buffer }
    const result = await encode(test).toPromise()

    const reverse = await decode(result).toPromise()
    expect(reverse).toEqual(test)
  })

  it('should serialize json containing valid IPLD links', async () => {
    // check that valid links will be encoded as CIDs
    const test = { link: { '/': 'QmZsm7qTeTTaF2K4uAKhP3b7zdRQwyv57Q4uD9JVyR2hiP' } }
    const result = await encode(test).toPromise()

    const reverse = await decode(result).toPromise()
    expect(reverse.link).toEqual({ '/': 'QmZsm7qTeTTaF2K4uAKhP3b7zdRQwyv57Q4uD9JVyR2hiP' })
  })

  it('should not serialize json containing invalid IPLD links', async () => {
    // check that basic sanity checks are performed on IPLD links
    const test = { link: { '/': 'QmZsm7qTeTTaF2K4uAKhP3b7zdRQwyv57Q4uD9JVyR2h' } }
    const result = await encode(test)
      .toPromise()
      .catch(reason => reason.toString())
    expect(result).toMatch(/multihash length inconsistent/)
  })

  it('should not serialize functions', async () => {
    // check that attempts to serialize functions fail fast instead of producing gibberish
    const test = () => 'BOOM'
    const result = await encode(test)
      .toPromise()
      .catch(reason => reason.toString())
    expect(result).toMatch(/Unknown type: function/)
  })

  it('should not deserialize invalid dag-cbor functions', async () => {
    // check that invalid cbor leads to a failed observable
    const buffer = Buffer.from('most likely not valid dag-cbor')
    const result = await decode(buffer)
      .toPromise()
      .catch(reason => reason.toString())
    expect(result).toMatch(/Error: Failed to parse/)
  })
  it.skip(
    'should encode and decode quickly (hot)',
    async () => {
      const buffer = fs.readFileSync('testdata/periblock.json.gz')
      const data: pako.Data = new Uint8Array(buffer)
      const text = pako.inflate(data, { to: 'string' })
      const testData = JSON.parse(text)
      // warmup (JIT)
      for (let j = 0; j < 100; j++) {
        const cbor = await encode(testData).toPromise()
        const json = JSON.stringify(testData)
        const reverse1 = await decode(cbor).toPromise()
        const reverse2 = JSON.parse(json)
        expect(reverse1).toEqual(reverse2)
      }
      const showResult = (name: string, dt: number, l: number): void => {
        console.info(`${name} ${dt / 1e3} ms ${l} bytes ${(l * 1e6) / dt} bytes/s`)
      }
      // benchmark
      for (let j = 0; j < 1; j++) {
        const t0 = Timestamp.now()
        const cbor = await encode(testData).toPromise()
        const t1 = Timestamp.now()
        const json = JSON.stringify(testData)
        const t2 = Timestamp.now()
        const reverse1 = await decode(cbor).toPromise()
        const t3 = Timestamp.now()
        const reverse2 = JSON.parse(json)
        const t4 = Timestamp.now()
        showResult('cbor encode', t1 - t0, cbor.length)
        showResult('json encode', t2 - t1, json.length)
        showResult('cbor decode', t3 - t2, cbor.length)
        showResult('json decode', t4 - t3, json.length)
        expect(reverse1).toEqual(reverse2)
      }
    },
    20000,
  )
})
