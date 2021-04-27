import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import { Hero } from '../components/Hero'
import { HowPageIsStructured } from '../components/HowPageIsStructured'
import { ActyxEssentials } from '../components/ActyxEssentials'
// import { TypeForm } from '../icons/icons'
import useBaseUrl from '@docusaurus/useBaseUrl'
import { NPS } from '../components/NPS'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1200px;
  background-color: #ffffff;
  margin-left: auto;
  margin-right: auto;
`
const customStyle = {
  border: 'none',
  height: '54px',
  position: 'fixed',
  boxShadow: '0px 2px 12px rgba(0, 0, 0, 0.26), 0px 2px 4px rgba(0, 0, 0, 0.28)',
  right: '26px',
  bottom: '26px',
  borderRadius: '5px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  cursor: 'pointer',
  background: '#369AFF',
  overflow: 'hidden',
  lineHeight: 0,
}

function Home() {
  return (
    <>
      <PageWrapper>
        <Hero img={useBaseUrl('/images/homepage/hero-background.svg')} />
        <HowPageIsStructured />
        <ActyxEssentials img={useBaseUrl('/images/homepage/homepage-links.svg')} />
        <NPS />
      </PageWrapper>
    </>
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
