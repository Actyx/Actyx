import React from 'react'
import styled from 'styled-components'
import { Card } from '../components/Card'
import { Headline } from '../components/Headline'
import { Body } from '../components/Body'

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
  padding-bottom: 24px;
`
const Cards = styled.div`
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

type Props = Readonly<{
  example: string
}>

export const HowPageIsStructured: React.FC<Props> = ({ example }: Props) => (
  <Wrapper>
    <Header>
      <Headline headline="How this website is structured" />
      <Body body="Actyx has a lot of documentation and it is easy to get lost in them. This is why we organised our docs in a way that makes it easy for you to find exactly what you are looking for. This high-level overview will help you know where to look for certain docs. " />
    </Header>
    <Cards>
      <Card
        color="blue"
        headline="How-to Guides"
        body="These problem-oriented guides take the reader through a series of steps required to solve a problem."
        cta="Find out more"
        link="/docs/how-to/overview"
      />
      <Card
        color="green"
        headline="Conceptual Guides"
        body="These understanding-oriented guides clarify a particular topic by giving context and a wider view."
        cta="Find out more"
        link="/docs/conceptual/overview"
      />
      <Card
        color="purple"
        headline="Reference Docs"
        body="These information-oriented docs provide technical descriptions of the code and how to operate it."
        cta="Find out more"
        link="/docs/reference/overview"
      />
      <Card
        color="orange"
        headline="Tutorials"
        body="These learning-oriented lessons take the reader by the hand to complete a small project."
        cta="Find out more"
        link="/docs/tutorials/overview"
      />
    </Cards>
  </Wrapper>
)
