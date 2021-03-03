import React from 'react'
import styled from 'styled-components'
import { keyframes } from 'styled-components'

const Wrapper = styled.div``

const waveAnimation = keyframes`
0% {
    transform: rotate(0deg);
  }
  10% {
    transform: rotate(14deg);
  }
  20% {
    transform: rotate(-8deg);
  }
  30% {
    transform: rotate(14deg);
  }
  40% {
    transform: rotate(-4deg);
  }
  50% {
    transform: rotate(10deg);
  }
  60% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(0deg);
  }
`

const Test = styled.div`
  width: 60px;
  height: 60px;
  font-size: 42px;
  position: relative;
  animation-name: ${waveAnimation};
  animation-duration: 4s;
  animation-iteration-count: infinite;
`

type Props = Readonly<{
  example: string
}>

export const PageTag: React.FC<Props> = ({ example }: Props) => (
  <Wrapper>
    <Test>👋</Test>
  </Wrapper>
)
