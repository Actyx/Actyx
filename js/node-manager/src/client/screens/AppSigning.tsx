import React, { useState } from 'react'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Layout } from '../components/Layout'
import { Button, SimpleInput } from '../components/basics'
import { isNone } from 'fp-ts/lib/Option'
import { getFileFromUser } from '../util'
import { Wizard, WizardFailure, WizardSuccess, WizardInput } from '../util'
import { Either, right, left } from 'fp-ts/lib/Either'
import { sleep } from '../../common/util'
import { signAppManifest } from '../util'
import { useAppState } from '../app-state/app-state'

const Screen = () => {
  const {
    actions: { signAppManifest: createSignedAppManifest },
  } = useAppState()
  const execute = async (input: Input): Promise<Either<string, null>> =>
    signAppManifest(input)
      .then(() => right(null))
      .catch((e) => left(e.shortMessage))

  return (
    <Layout title="App Signing">
      <SimpleCanvas>
        <Wizard failure={Failed} success={Succeeded} input={Initial} execute={execute} />
      </SimpleCanvas>
    </Layout>
  )
}

interface Input {
  pathToManifest: string
  pathToCertificate: string
}

const Initial: WizardInput<Input> = ({ execute, executing }) => {
  const [manifestFile, setManifestFile] = useState('')
  const [certificateFile, setCertificateFile] = useState('')

  const selectFile = async (onGet: (path: string) => void, exts?: string[]) => {
    const file = await getFileFromUser(exts)
    if (isNone(file)) {
      return
    }
    onGet(file.value)
  }

  const doExecute = () => {
    if (manifestFile === '' || certificateFile === '') {
      return
    }
    execute({ pathToManifest: manifestFile, pathToCertificate: certificateFile })
  }

  return (
    <>
      <p>Sign your manifest using a developer certificate.</p>
      <p>
        <em>Note that this will overwrite any existing signature in the manifest file.</em>
      </p>
      <div className="p-2">
        <SimpleInput
          className="mt-4"
          label="Path to manifest"
          placeholder="Select path to manifest"
          value={manifestFile}
          disabled={true}
          button={{
            text: 'Select file',
            onClick: () => selectFile(setManifestFile, ['json']),
            disabled: executing,
          }}
        />
        <SimpleInput
          className="mt-4"
          label="Path to developer certificate"
          placeholder="Select path to developer certificate"
          value={certificateFile}
          disabled={true}
          button={{
            text: 'Select file',
            onClick: () => selectFile(setCertificateFile, ['json']),
            disabled: executing,
          }}
        />
        <div className="flex mt-8">
          <Button
            onClick={doExecute}
            disabled={manifestFile === '' || certificateFile === ''}
            working={executing}
          >
            Sign manifest
          </Button>
        </div>
      </div>
    </>
  )
}

const Succeeded: WizardSuccess<null> = ({ restart }) => (
  <>
    <p className="mb-2">Successfully signed manifest.</p>
    <Button className="mt-8" onClick={restart}>
      Back
    </Button>
  </>
)

const Failed: WizardFailure<string> = ({ restart, reason }) => (
  <>
    <p className="text-red-500 font-medium mb-2">Error signing manifest</p>
    <p>{reason}</p>
    <Button className="mt-8" onClick={restart}>
      Back
    </Button>
  </>
)

export default Screen
