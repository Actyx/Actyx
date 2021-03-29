import React from 'react'
import styled from 'styled-components'
import { GitHub, LinkedIn, Twitter } from '../icons/icons'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  width: 80px;
  margin-bottom: 8px;
`

type Props = Readonly<{
  example: string
}>

export const Social: React.FC<Props> = ({ example }: Props) => (
  <Wrapper>
    <a href="https://github.com/actyx">
      <GitHub color="lightgray" positive />
    </a>
    <a href="https://www.linkedin.com/company/10114352">
      <LinkedIn color="lightgray" positive />
    </a>
    <a href="https://www.twitter.com/actyx">
      <Twitter color="lightgray" positive />
    </a>
  </Wrapper>
)
