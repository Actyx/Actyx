import React from 'react'
import styled from 'styled-components'
import { keyframes } from 'styled-components'
import SearchBarHomePage from '../theme/SearchBar-Homepage'

const Wrapper = styled.div`
  margin-left: auto;
  margin-right: auto;
  padding-top: 64px;
`

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

const LineWrapper = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: center;
`

const WaveAnimation = styled.div`
  width: 60px;
  font-size: 26px;
  position: relative;
  margin-left: 4px;
  animation-name: ${waveAnimation};
  animation-duration: 4s;
  animation-iteration-count: infinite;
`

const HeroHeadline = styled.div`
  max-width: 650px;
  font-size: 26px;
  font-weight: 900;
  text-align: center;
`

const HeroCopy = styled.div`
  font-size: 15px;
  max-width: 450px;
  text-align: center;
  margin-top: 16px;
  margin-bottom: 24px;
`

export const Hero: React.FC = () => (
  <Wrapper>
    <LineWrapper>
      <HeroHeadline>Hey there!</HeroHeadline>
      <WaveAnimation>👋</WaveAnimation>
    </LineWrapper>
    <LineWrapper>
      <HeroHeadline>Welcome to the Actyx developer documentation.</HeroHeadline>
    </LineWrapper>
    <LineWrapper>
      <HeroCopy>
        The place where you find everything you need to digitize factory processes and build
        <a href="https://local-first-cooperation.github.io/website/"> local-first </a> solutions on
        the Actyx platform.
      </HeroCopy>
    </LineWrapper>
    <LineWrapper>
      <SearchBarHomePage />
    </LineWrapper>
  </Wrapper>
)
