import React from 'react'
import styled from 'styled-components'
import renderer from '../components/Renderer'
import Markdown from 'react-markdown'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  overflow: auto;
  height: auto;
  height: 600px;
  border-bottom: 1px solid gray;
`
const ExplanationWrapper = styled.div`
  flex-basis: 50%;
  margin-right: 14px;
  height: 600px;
`

const CodeWrapper = styled.div`
  flex-basis: 50%;
  margin-left: 14px;
  position: sticky;
  position: -webkit-sticky;
  align-self: flex-start;
  height: auto;
  top: 0;
`

type Props = Readonly<{
  code: string
  explanation: React.ReactNode
}>

export const APIReference: React.FC<Props> = ({ explanation, code }: Props) => (
  <Wrapper>
    <ExplanationWrapper>
      <p>
        You can get information from the Event Service about known offsets, i.e. what the event
        service believes to be the latest offset for each stream.
        <br /> Take a look at the Event Streams guide to learn more about the role of offsets.
      </p>
      <h3>Request</h3>
      <ul>
        <li>Endpoint: http://localhost:4454/api/v2/events/offsets</li>
        <li>HTTP method: GET</li>
        <li>HTTP headers:</li>
        <ul>
          <li>Authorization, see Prerequisites</li>
          <li>(optional) Accept, must be application/json, default: application/json</li>
        </ul>
      </ul>
      <p>There is no request body.</p>
      <h3>Response</h3>
      <ul>
        <li>HTTP headers:</li>
        <ul>
          <li>Content-Type is `application/json`</li>
          <li>Cache-Control is no-store (to get fresh data and not use cache slots)</li>
        </ul>
      </ul>
      <p>The response body will contain a JSON object of the following structure:</p>
    </ExplanationWrapper>
    <CodeWrapper>
      <Markdown renderers={renderer} source={code} />
    </CodeWrapper>
  </Wrapper>
)
