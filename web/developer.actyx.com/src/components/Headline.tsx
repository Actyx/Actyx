import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  font-size: 26px;
  color: #303c4b;
  margin-bottom: 16px;
`

type Props = Readonly<{
  headline: string
}>

export const Headline: React.FC<Props> = ({ headline }: Props) => <Wrapper>{headline}</Wrapper>
