import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import { PageTag } from '../components/PageTag'
import { APIReference } from '../components/APIReference'
import { Callout } from '../components/Callout'
import { Hero } from '../components/Hero'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1200px;
  height: 50vh;
  background-color: #ffffff;
  margin-top: 40px;
  margin-left: auto;
  margin-right: auto;
`

function Home() {
  return (
    <PageWrapper>
      <Hero/>
    </PageWrapper>
  )
}

function HomePage() {
  return (
    <Layout title="Actyx Developer Documentation" description="">
      <Home />
    </Layout>
  )
}

export default HomePage
