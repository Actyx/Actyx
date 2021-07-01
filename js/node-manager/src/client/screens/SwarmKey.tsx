import { SimpleCanvas } from '../components/SimpleCanvas'
import React, { useState } from 'react'
import { Layout } from '../components/Layout'
import { Button, SimpleInput } from '../components/basics'
import { useAppState } from '../app-state'
import { saveToClipboard } from '../util'
import { Wizard, WizardFailure, WizardSuccess, WizardInput } from '../util'
import { Either, left, right } from 'fp-ts/lib/Either'
import { ClipboardCheckedIcon, ClipboardIcon } from '../components/icons'

const Screen = () => {
  const {
    actions: { generateSwarmKey },
  } = useAppState()

  const execute = (): Promise<Either<string, Success>> =>
    generateSwarmKey()
      .then((resp) => right(resp))
      .catch((e) => left(e))

  return (
    <Layout title="Swarm Key Generator">
      <SimpleCanvas>
        <Wizard failure={Failed} success={Success} input={Initial} execute={execute} />
      </SimpleCanvas>
    </Layout>
  )
}

interface Success {
  swarmKey: string
}

const Initial: WizardInput<{}> = ({ execute, executing }) => {
  return (
    <>
      <p>Generate a swarm key to secure a swarm of Actyx nodes.</p>
      <div className="p-2 pt-0">
        <div className="flex mt-8">
          <Button onClick={execute} working={executing}>
            Generate swarm key
          </Button>
        </div>
      </div>
    </>
  )
}

const Success: WizardSuccess<Success> = ({ restart, result: { swarmKey } }) => {
  const [copiedToClipboard, setCopiedToClipboard] = useState(false)
  const toClipboard = () => {
    saveToClipboard(swarmKey)
    setCopiedToClipboard(true)
  }

  return (
    <>
      <p className="mb-0">Successfully generated swarm key.</p>
      <div className="p-2">
        <SimpleInput
          className="mt-4"
          label="Generated swarm key"
          value={swarmKey}
          disabled
          inputClassName="text-sm text-gray-600"
        />
        <div className="mt-8 flex flex-row">
          <Button onClick={restart}>Back</Button>
          <Button
            className="ml-3"
            onClick={toClipboard}
            icon={!copiedToClipboard ? <ClipboardIcon /> : <ClipboardCheckedIcon />}
          >
            Copy to clipboard
          </Button>
        </div>
      </div>
    </>
  )
}

const Failed: WizardFailure<string> = ({ restart, reason }) => (
  <>
    <p className="text-red-500 font-medium mb-2">Error generating swarm key</p>
    <p>{reason}</p>
    <Button className="mt-8" onClick={restart}>
      Back
    </Button>
  </>
)

export default Screen
