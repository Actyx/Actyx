import React, { useEffect, useState } from 'react'
import { useCookies } from 'react-cookie'
import styled from 'styled-components'

const DONT_SHOW_COOKIE_NAME = '_ax-dev-nps_dont-show'
const DONT_SHOW_FOR_SECONDS_AFTER_SUBMISSION = 15 * 24 * 60 * 60 // 15 days
const DONT_SHOW_FOR_SECONDS_AFTER_CLICKING_AWAY = 7 * 24 * 60 * 60 // 7 days
const WAIT_TO_SHOW_SECONDS = 60 // Wait 1 minute before showing
const HIDE_AFTER_SECONDS = 5 * 60 // Wait 5 minutes before hiding

const Anchor = styled.div`
  position: relative;
  z-index: 1000;
`

const Wrapper = styled.div<{ visible: boolean }>`
  transition: all 0.5s ease-in-out;
  position: fixed;
  bottom: 0;
  transform: translateY(${(props) => (props.visible ? '0' : '100%')});
  width: 100%;
  background-color: #f5f6f7;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 1.5rem;
  box-shadow: 0 30px 70px -12px rgba(50, 50, 93, 0.9), 0 18px 36px -18px rgba(0, 0, 0, 0.5);
`

const Heading = styled.p`
  font-weight: 600;
  font-size: 120% !important;
  text-align: center;
  margin-bottom: 10px;
`
const Text = styled.p`
  font-weight: normal;
  font-size: 120% !important;
  text-align: center;
`
const Boxes = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: center;
  flex-wrap: wrap;
`

const Box = styled.div`
  width: 40px;
  height: 40px;
  margin-left: 0.2rem;
  margin-right: 0.2rem;
  margin-top: 0.2rem;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  background-color: #dfeaf3;
  transition: all 0.2s ease-in-out;
  &:hover {
    background-color: var(--ifm-link-color);
    color: #fff;
  }
`

type Props = Readonly<{
  showAfterMs?: number
  hideAfterMs?: number
}>

const submitResult = async (result: number): Promise<void> => {
  const res = await fetch('/.netlify/functions/nps', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ result }),
  })

  if (res.status !== 200) {
    console.error(`submission did not return status code 200 (got ${res.status})`)
    throw new Error(res.statusText)
  }
}

type State = 'not-shown-yet' | 'visible' | 'clicked-away' | 'faded-away' | 'submitted'

const QuestionView: React.FC<{ clickAway: () => void; submit: (result: number) => void }> = ({
  clickAway,
  submit,
}) => (
  <>
    <Heading>
      On a scale from 0-10, how likely are you to recommend Actyx to a friend or colleague?{' '}
      <a href="#" onClick={clickAway}>
        Close.
      </a>
    </Heading>
    <Boxes>
      {[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((v) => (
        <Box key={v} onClick={() => submit(v)}>
          {v}
        </Box>
      ))}
    </Boxes>
  </>
)

const ThankYouView: React.FC<{ clickAway: () => void }> = ({ clickAway }) => (
  <>
    <Heading>Thank you for your feedback!</Heading>
    <Text>
      If you would like to make further comments or suggestions, we invite you to do so in our{' '}
      <a href="https://community.actyx.com" target="_blank" rel="noopener noreferrer">
        Community Forum
      </a>
      .
    </Text>
    <Heading>
      <a href="#" onClick={clickAway}>
        Close.
      </a>
    </Heading>
  </>
)

export const NpsSurvey = ({ showAfterMs, hideAfterMs }: Props): React.ReactElement => {
  showAfterMs = showAfterMs === undefined ? WAIT_TO_SHOW_SECONDS * 1000 : showAfterMs
  hideAfterMs = hideAfterMs === undefined ? HIDE_AFTER_SECONDS * 1000 : hideAfterMs

  const [cookies, setCookie] = useCookies([DONT_SHOW_COOKIE_NAME])

  const [state, setState] = useState<State>('not-shown-yet')

  useEffect(() => {
    let showTimer = null
    if (state === 'not-shown-yet' && !cookies[DONT_SHOW_COOKIE_NAME]) {
      showTimer = setTimeout(() => {
        setState('visible')
      }, showAfterMs)
    }

    const hideTimer = setTimeout(() => {
      setState((current) => {
        if (current === 'visible') {
          return 'faded-away'
        }
      })
    }, showAfterMs + hideAfterMs)

    return () => {
      if (showTimer !== null) {
        clearTimeout(showTimer)
      }
      clearTimeout(hideTimer)
    }
  }, [])

  const disableForSec = (seconds: number) => {
    setCookie(DONT_SHOW_COOKIE_NAME, new Date().toISOString(), {
      maxAge: seconds,
    })
  }

  const clickAway = () => {
    // Check if clicked away from visible since this can also be
    // called from the thank you screen.
    if (state === 'visible') {
      disableForSec(DONT_SHOW_FOR_SECONDS_AFTER_CLICKING_AWAY)
    }
    setState('clicked-away')
  }

  const submit = (result: number) => {
    submitResult(result)
    setState('submitted')
    disableForSec(DONT_SHOW_FOR_SECONDS_AFTER_SUBMISSION)
  }

  return (
    <Anchor>
      <Wrapper visible={state === 'visible' || state === 'submitted'}>
        {state === 'visible' && <QuestionView clickAway={clickAway} submit={submit} />}
        {state === 'submitted' && <ThankYouView clickAway={clickAway} />}
      </Wrapper>
    </Anchor>
  )
}
