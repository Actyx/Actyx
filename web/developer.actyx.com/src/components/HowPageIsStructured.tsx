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
  padding: 24px;
`
const Cards = styled.div`
  display: flex;
  flex-direction: row;
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
        color="green"
        headline="Conceptual Guides"
        body="These understanding-oriented guides clarify a particular topic by giving context and a wider view."
        cta="Find out more"
        link="/releases"
      />
      <Card
        color="blue"
        headline="How-to Guides"
        body="These problem-oriented guides take the reader through a series of steps required to solve a problem."
        cta="Find out more"
        link="/releases"
      />
      <Card
        color="purple"
        headline="Reference Docs"
        body="These information-oriented docs provide technical descriptions of the code and how to operate it."
        cta="Find out more"
        link="/releases"
      />
      <Card
        color="orange"
        headline="Tutorials"
        body="These learning-oriented lessons take the reader by the hand through a series of steps to complete a project."
        cta="Find out more"
        link="/releases"
      />
    </Cards>
  </Wrapper>
)
