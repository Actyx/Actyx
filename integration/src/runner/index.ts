import { readFileSync } from 'fs'

const configFile = process.env.NODES || 'nodes.json'
const config = JSON.parse(readFileSync(configFile).toString())
