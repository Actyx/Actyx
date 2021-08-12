import React, { useState } from 'react'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Layout } from '../components/Layout'
import { Button, SimpleInput } from '../components/basics'
import { Wizard, WizardFailure, WizardSuccess, WizardInput } from '../util'
import { Either, left, right } from 'fp-ts/lib/Either'
import { useAppState, AppActionKey } from '../app-state'
import { CreateUserKeyPairResponse } from '../../common/types'
import { validate_private_key } from 'ax-wasm'

const DefaultDirectoryHelpLink: React.FC = ({ children }) => (
  <a
    className="underline text-blue-500"
    target="_blank"
    rel="noopener noreferrer"
    href="https://developer.actyx.com/docs/how-to/user-auth/set-up-user-keys"
  >
    {children}
  </a>
)

const Initial: WizardInput<string | undefined> = ({ execute, executing }) => {
  const [privKey, setPrivKey] = useState('')
  return (
    <>
      <p className="text-xl pb-6">No user key pair found</p>

      <p className="pb-10 text-gray-400">
        The Actyx Node Manager needs a user key pair to authenticate itself with Actyx nodes. If you
        already have a key pair, please provide it here:
      </p>

      <SimpleInput
        className="mt-4"
        label="Enter private key"
        placeholder="Private key"
        setValue={setPrivKey}
        value={privKey}
        button={{
          text: 'Ok',
          onClick: () => execute(privKey),
          disabled: executing,
        }}
        disabled={executing}
      />
      <div className="mt-10">
        <Button onClick={() => execute(undefined)} working={executing}>
          Create new user key pair
        </Button>
      </div>
    </>
  )
}

const mkSuccess =
  (onDone: () => void): WizardSuccess<CreateUserKeyPairResponse> =>
  ({ result: { privateKey } }) =>
    (
      <>
        <p className="text-xl pb-6">User key pair created</p>
        <p className="text-gray-400">The Actyx Node Manager created a user key pair for you.</p>

        <SimpleInput
          className="mt-6"
          label="Your private key is"
          value={privateKey}
          disabled
          inputClassName="text-sm text-gray-600"
        />
        <Button className="mt-8" onClick={onDone}>
          Ok
        </Button>
      </>
    )

const Failure: WizardFailure<string> = ({ restart, reason }) => (
  <>
    <p className="text-xl text-red-500 font-medium pb-6">Error creating user key pair</p>

    <p className="pb-10 text-gray-400">{reason}</p>
    <Button onClick={restart}>Try again</Button>
  </>
)

const Screen = () => {
  const {
    dispatch,
    actions: { createUserKeyPair, setUserKeyPair },
  } = useAppState()

  const execute = async (
    maybeInput?: string,
  ): Promise<Either<string, CreateUserKeyPairResponse>> => {
    if (maybeInput) {
      try {
        validate_private_key(maybeInput)
        setUserKeyPair(maybeInput)
        return right({ privateKey: maybeInput })
      } catch (e) {
        return left(e)
      }
    } else {
      return await createUserKeyPair()
        .then((r) => right(r))
        .catch((e) => left(e))
    }
  }
  return (
    <Layout title="Setup a user key">
      <SimpleCanvas>
        <Wizard
          failure={Failure}
          success={mkSuccess(() => dispatch({ key: AppActionKey.ShowOverview }))}
          input={Initial}
          execute={execute}
        />
      </SimpleCanvas>
    </Layout>
  )
}

export default Screen
