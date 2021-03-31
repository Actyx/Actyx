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
`

const Header = styled.div`
  max-width: 50%;
  padding-left: 0px;
  padding-right: 24px;
  padding-top: 24px;
  padding-bottom: 0px;
`
const LinkWrapper = styled.div`
  display: grid;
  grid-auto-columns: 1fr;
  grid-auto-flow: column;
  column-gap: 24px;
  row-gap: 24px;
  &:first-child {
    grid-column-start: start;
    grid-column-end: line2;
  }
  &:nth-child(2) {
    grid-column-start: line2;
    grid-column-end: line3;
  }
  &:nth-child(3) {
    grid-column-start: line3;
    grid-column-end: line4;
  }
  &:nth-child(4) {
    grid-column-start: line4;
    grid-column-end: end;
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
type Props = Readonly<{
  img: string
}>

export const ActyxEssentials: React.FC<Props> = ({ img }: Props) => (
  <Wrapper>
    <Header>
      <Headline headline="Actyx essentials" />
      <Body body="Check out the following topics to learn the essentials to know how to build, run, and deploy your solution to your factory customers." />
    </Header>
    <LinkWrapper>
      <Links color="green">
        <Link title="Event-based systems" link="" color="green" positive />
        <Link title="Thinking in Actyx" link="" color="green" positive />
        <Link title="Peer Discovery" link="" color="green" positive />
        <Link title="Performance and Limitations" link="" color="green" positive />
      </Links>
      <Links color="blue">
        <Link title="Installing and starting Actyx" link="" color="blue" positive />
        <Link title="Modelling processes in local twins" link="" color="blue" positive />
        <Link title="Some SDK Guide" link="" color="blue" positive />
        <Link title="Packaging Front-End Apps" link="" color="blue" positive />
      </Links>
      <Links color="purple">
        <Link title="Actyx API Reference" link="" color="purple" positive />
        <Link title="Actyx SDK Reference" link="" color="purple" positive />
        <Link title="CLI Commands Reference" link="" color="purple" positive />
        <Link title="Node Manager Features" link="" color="purple" positive />
      </Links>
      <Card
        color="white"
        headline="Actyx Community"
        body="Join our developer community and discover our forum."
        cta="Forum"
        link="/"
      />
      {/* <Image img={img} /> */}
    </LinkWrapper>
  </Wrapper>
)
