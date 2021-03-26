import React from 'react'
import styled from 'styled-components'
import { keyframes } from 'styled-components'
import SearchBarHomePage from '../theme/SearchBar-Homepage'
import { Card } from '../components/Card'

const Wrapper = styled.div`
  margin-left: auto;
  margin-right: auto;
  width: 100%;
  padding-top: 36px;
  padding-bottom: 36px;
  background-color: #ffffff;
  opacity: 1;
  background-image: radial-gradient(#c6cdd8 1px, #ffffff 1px);
  background-size: 20px 20px;
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
  background: white;
`

const HeroCopy = styled.div`
  font-size: 15px;
  max-width: 450px;
  text-align: center;
  margin-top: 16px;
  margin-bottom: 24px;
  background: white;
`
const CardWrapper = styled.div`
  margin-left: auto;
  margin-right: auto;
  margin-top: 32px;
  display: flex;
  max-width: 55%;
  flex-direction: row;
  justify-content: center;
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
    <CardWrapper>
      <Card
        headline="Ready to get started?"
        body="Write your first local twin in only 10 minutes!"
        link="/"
        color="purple"
        cta="Quick Start"
      />
      <Card
        headline="New to Actyx?"
        body="Learn the basics of Actyx and see its benefits!"
        link="/"
        color="blue"
        cta="How Actyx works"
      />
    </CardWrapper>
  </Wrapper>
)
