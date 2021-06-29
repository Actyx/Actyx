import { SimpleCanvas } from '../components/SimpleCanvas'
import React from 'react'
import { Layout } from '../components/Layout'
import { useAppState } from '../app-state'

const DocsLink: React.FC = ({ children }) => (
  <a
    className="underline text-blue-500"
    target="_blank"
    rel="noopener noreferrer"
    href="https://developer.actyx.com"
  >
    {children}
  </a>
)

const ForumLink: React.FC = ({ children }) => (
  <a
    className="underline text-blue-500"
    target="_blank"
    rel="noopener noreferrer"
    href="https://community.actyx.com"
  >
    {children}
  </a>
)

const ActyxLink: React.FC = ({ children }) => (
  <a
    className="underline text-blue-500"
    target="_blank"
    rel="noopener noreferrer"
    href="https://www.actyx.com"
  >
    {children}
  </a>
)

const Screen: React.FC<{}> = () => {
  const {
    state: { version },
  } = useAppState()
  return (
    <Layout title="About">
      <SimpleCanvas>
        <div className="flex flex-col flex-grow flex-shrink">
          <p className="text-xl pb-6 flex-grow-0 flex-shrink-0">Actyx Node Manager</p>
          <p className="text-gray-400 pb-3 flex-grow-0 flex-shrink-0">
            Problems or feedback? Visit our <DocsLink>Developer Documentation</DocsLink> or get in
            touch on our <ForumLink>Community Forum</ForumLink>.
          </p>
          <div className="text-gray-400 text-sm flex-grow flex-shrink flex flex-col justify-end mb-3">
            <p className="pb-3">
              By using this software you agree to the Actyx Software License Agreement, the Actyx
              Developer Terms and the Actyx Privacy Policy which you can find at{' '}
              <ActyxLink>www.actyx.com</ActyxLink>.
            </p>
            <p>Version {version}. Built by Actyx AG.</p>
          </div>
        </div>
      </SimpleCanvas>
    </Layout>
  )
}

export default Screen
