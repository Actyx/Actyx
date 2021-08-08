import React from 'react'
import { Redirect } from '@docusaurus/router'

const Page: React.FC<{
  to: string
}> = ({ to }) => <Redirect to={to} />

export default Page
