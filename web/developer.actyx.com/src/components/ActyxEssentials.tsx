import React from 'react'
import styled from 'styled-components'
import { Card } from '../components/Card'
import { Headline } from '../components/Headline'
import { Body } from '../components/Body'
import { Link } from './Link'

const Wrapper = styled.div`
  display: flex;
  flex-direction: column;
  margin-bottom: 36px;
  padding-left: 36px;
  padding-right: 36px;
`

const Header = styled.div`
  max-width: 50%;
  padding-left: 0px;
  padding-right: 24px;
  padding-top: 24px;
  padding-bottom: 0px;
  @media (max-width: 996px) {
    max-width: 100%;
  }
`
const LinkWrapper = styled.div`
  display: grid;
  grid-auto-columns: 1fr;
  grid-auto-flow: column;
  column-gap: 24px;
  row-gap: 24px;
  @media (max-width: 996px) {
    grid-column-start: start;
    grid-column-end: end;
    grid-auto-flow: row;
  }
`

const Links = styled.div<{
  color?: string
}>`
  border-radius: 4px;
  color: ${(p) =>
    p.color == 'green'
      ? '#15BE53'
      : p.color == 'blue'
      ? '#369AFF'
      : p.color == 'purple'
      ? '#635BFF'
      : p.color == 'orange'
      ? '#FF9933'
      : '#f5f5f5'};
  padding-bottom: 12px;
  padding-top: 18px;
`
const Image = styled.div<{
  img: string
}>`
  background: #15be53;
  background: url(${(props) => props.img}) no-repeat center;
  background-size: 260px;
`

export const ActyxEssentials: React.FC = () => (
  <Wrapper>
    <Header>
      <Headline headline="Actyx Essentials" />
      <Body body="Check out the following topics to learn the essentials of building, running, and deploying systems using Actyx" />
    </Header>
    <LinkWrapper>
      <Links color="blue">
        <Link
          title="Install and start Actyx"
          link="/docs/how-to/local-development/install-actyx"
          color="blue"
          positive
        />
        <Link
          title="Set up your environment"
          link="/docs/how-to/local-development/set-up-your-environment"
          color="blue"
          positive
        />
        <Link
          title="Set up a new project"
          link="/docs/how-to/local-development/set-up-a-new-project"
          color="blue"
          positive
        />
        <Link
          title="Package for mobile"
          link="/docs/how-to/packaging/mobile-apps"
          color="blue"
          positive
        />
        <Link title="Set up a swarm" link="/docs/how-to/swarms/setup-swarm" color="blue" positive />
      </Links>
      <Links color="green">
        <Link
          title="Event-based systems"
          link="/docs/conceptual/event-based-systems"
          color="green"
          positive
        />
        <Link title="Actyx Jargon" link="/docs/conceptual/actyx-jargon" color="green" positive />
        <Link
          title="The Actyx Node"
          link="/docs/conceptual/the-actyx-node"
          color="green"
          positive
        />
        <Link
          title="Performance and Limitations"
          link="/docs/conceptual/performance-and-limits-of-actyx"
          color="green"
          positive
        />
      </Links>
      <Links color="purple">
        <Link
          title="Actyx API Reference"
          link="/docs/reference/actyx-reference"
          color="purple"
          positive
        />
        <Link
          title="Event Service Reference"
          link="/docs/reference/event-service"
          color="purple"
          positive
        />
        <Link
          title="CLI Reference"
          link="/docs/reference/cli/cli-overview"
          color="purple"
          positive
        />
        <Link
          title="Node Manager Features"
          link="/docs/reference/node-manager"
          color="purple"
          positive
        />
      </Links>
      <Card
        color="white"
        headline="Actyx Community"
        body="Join our developer community and discover our forum."
        cta="Forum"
        link="https://community.actyx.com/"
      />
    </LinkWrapper>
  </Wrapper>
)
