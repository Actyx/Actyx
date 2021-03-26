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

export const ActyxEssentials: React.FC<Props> = ({ example }: Props) => (
  <Wrapper>
    <Header>
      <Headline headline="Actyx essentials" />
      <Body body="Check out the following topics to learn the essentials to know how to build, run, and deploy your solution to your factory customers." />
    </Header>
  </Wrapper>
)
