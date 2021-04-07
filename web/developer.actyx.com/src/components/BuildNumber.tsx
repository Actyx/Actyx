import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: center;
  display: inline-block;
  margin-bottom: 12px;
  text-align: center;
  font-family: SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-size: 13px;
  padding-left: 12px;
  padding-right: 12px;
  padding-top: 4px;
  padding-bottom: 4px;
  border-radius: 5px;
  background-color: #1e1e1e;
`

const Build = styled.div`
  background-color: #1e1e1e;
  color: #9cdcfe;
  margin-right: 8px;
  display: inline-block;
`

const BuildNr = styled.div`
  background-color: #1e1e1e;
  color: #b5cea8;
  display: inline-block;
`

type Props = Readonly<{
  pre: string
  build: string
}>

export const BuildNumber: React.FC<Props> = ({ pre, build }: Props) => (
  <Wrapper>
    <Build>{pre}</Build>
    <BuildNr>{build}</BuildNr>
  </Wrapper>
)
