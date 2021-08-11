import React from 'react'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Layout } from '../components/Layout'
import { Button, SimpleInput } from '../components/basics'
import { Wizard, WizardFailure, WizardSuccess, WizardInput } from '../util'
import { Either, left, right } from 'fp-ts/lib/Either'
import { useAppState, AppActionKey } from '../app-state'
import { CreateUserKeyPairResponse } from '../../common/types'

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

const Initial: WizardInput<void> = ({ execute, executing }) => (
  <>
    <p className="text-xl pb-6">No user key pair found</p>

    <p className="pb-10 text-gray-400">
      The Actyx Node Manager needs a user key pair to authenticate itself with Actyx nodes. If you
      already have a key pair, please save it in the{' '}
      <DefaultDirectoryHelpLink>default key pair directory</DefaultDirectoryHelpLink>.
    </p>
    <Button onClick={() => execute()} working={executing}>
      Create new user key pair
    </Button>
  </>
)

const mkSuccess =
  (onDone: () => void): WizardSuccess<CreateUserKeyPairResponse> =>
  ({ result: { privateKeyPath, publicKey, publicKeyPath } }) =>
    (
      <>
        <p className="text-xl pb-6">User key pair created</p>
        <p className="text-gray-400">The Actyx Node Manager created a user key pair for you.</p>
        <SimpleInput
          className="mt-6"
          label="Your private key was saved at"
          value={privateKeyPath}
          disabled
          inputClassName="text-sm text-gray-600"
        />
        <SimpleInput
          className="mt-4"
          label="Your public key was saved at"
          value={publicKeyPath}
          disabled
          inputClassName="text-sm text-gray-600"
        />
        <SimpleInput
          className="mt-4"
          label="Your public key is"
          value={publicKey}
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
    actions: { createUserKeyPair },
  } = useAppState()

  const execute = (): Promise<Either<string, CreateUserKeyPairResponse>> =>
    createUserKeyPair(null)
      .then((r) => right(r))
      .catch((e) => left(e))

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
