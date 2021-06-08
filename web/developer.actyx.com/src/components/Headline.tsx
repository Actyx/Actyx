import React from 'react'
import styled from 'styled-components'
import defaults from '../components/defaults'

const Wrapper = styled.div`
  font-size: 26px;
  font-weight: 600;
  color: ${defaults.colors.darkgray};
  margin-bottom: 16px;
`

type Props = Readonly<{
  headline: string
}>

export const Headline: React.FC<Props> = ({ headline }: Props) => <Wrapper>{headline}</Wrapper>
