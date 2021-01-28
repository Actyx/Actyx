import React from 'react'
import Layout from '@theme/Layout'
import { SectionHero } from '../../src/components/SectionHero.tsx'
import styled from 'styled-components'
import { TwoElementRow } from '../../src/components/TwoElementRow.tsx'
import { ThreeElementRow } from '../../src/components/ThreeElementRow.tsx'
import { StayInformed } from '../../src/components/StayInformed.tsx'
import useBaseUrl from '@docusaurus/useBaseUrl'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1140px;
  padding-left: 40px;
  padding-right: 40px;
  margin-left: auto;
  margin-right: auto;
`

const VerticalSpace = styled.div`
  height: 50px;
  width: 100%;
`

function Home() {
  return (
    <PageWrapper>
      <SectionHero
        img={useBaseUrl('images/home/hero.svg')}
        title={'Welcome to the Actyx Developer Website!'}
        subtitle={
          'On this website you can find everything you need to get started on the Actyx platform. Why not start with our quickstart guide?'
        }
        cta={'quickstart guide'}
        link={useBaseUrl('/docs/learn-actyx/quickstart')}
        button={true}
      />
      <VerticalSpace />
      <h2>First Steps</h2>
      Discover these two sections about the Actyx platform and its products to get a first glimpse
      into the Actyx world.
      <TwoElementRow
        img1={useBaseUrl('images/home/platform.svg')}
        img2={useBaseUrl('images/home/products.svg')}
        title1={'Introduction to the Actyx Platform'}
        title2={'Actyx Platform Products'}
        body1={'Discover the Actyx platform and get to know it from in and out.'}
        body2={'Learn more about ActyxOS and Actyx Pond and see how they interact.'}
        cta1={['Learn more']}
        cta2={['Learn more']}
        link1={[useBaseUrl('/docs/home/actyx_platform')]}
        link2={[useBaseUrl('/docs/home/actyx_products')]}
      />
      <VerticalSpace />
      <h2>Documentation</h2>
      Explore the Actyx products further and learn how to use them.
      <TwoElementRow
        img1={useBaseUrl('images/home/os-hero.svg')}
        img2={useBaseUrl('images/home/pond-hero.svg')}
        title1={'ActyxOS Documentation'}
        title2={'Actyx Pond Documentation'}
        body1={'Read ActyxOS Documentation and learn about the developer tooling we offer.'}
        body2={
          'Learn about the Actyx Pond programming model and how to write applications on top of it.'
        }
        cta1={['Introduction to ActyxOS', 'API Reference']}
        cta2={['Introduction to Actyx Pond', 'Learn Actyx Pond']}
        link1={[useBaseUrl('/docs/os/general/introduction'), useBaseUrl('/docs/os/api/overview')]}
        link2={[useBaseUrl('/docs/pond/introduction'), useBaseUrl('/docs/pond/guides/hello-world')]}
      />
      <VerticalSpace />
      <h2>Further Resources</h2>
      <ThreeElementRow
        img1={useBaseUrl('images/icons/blog.svg')}
        img2={useBaseUrl('images/icons/theoretical.svg')}
        img3={useBaseUrl('images/icons/faq.svg')}
        title1={'Blog Posts'}
        title2={'Theoretical Foundation'}
        title3={'FAQ'}
        body1={
          'Read the most recent news and learn new things about the Actyx platform from its creators.'
        }
        body2={
          "Learn about what's underlying ActyxOS - distributed systems, event sourcing and the CAP theorem."
        }
        body3={
          'Get first-hand support and answers to your most burning questions about the system.'
        }
        cta1={'Read blog posts'}
        cta2={'More info'}
        cta3={'Read FAQ'}
        link1={useBaseUrl('/blog')}
        link2={useBaseUrl('/docs/os/theoretical-foundation/distributed-systems')}
        link3={useBaseUrl('/docs/faq/supported-programming-languages')}
        showLinks={true}
      />
      <VerticalSpace />
      <h2>Getting Started</h2>
      The fastest way to see the Actyx platform in action is to check out our
      <span>&nbsp;</span>
      <a href={useBaseUrl('/docs/learn-actyx/quickstart')}>Quickstart Guide</a>. We also provide a
      half-day ramp up session with one of our engineers to get you started developing on the Actyx
      platform. If you are interested please <a href="https://www.actyx.com/contact">contact us</a>{' '}
      or join us in our <a href="https://discord.gg/262yJhc">Discord channel</a> and we will be
      happy to help you set up or answer any questions you might have.
      <VerticalSpace />
      <h2>Stay informed</h2>
      <StayInformed
        img1={useBaseUrl('images/social/twitter.svg')}
        img2={useBaseUrl('images/social/github.svg')}
        img3={useBaseUrl('images/social/youtube.svg')}
        img4={useBaseUrl('images/social/linkedin.svg')}
      />
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
