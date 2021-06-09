import React from 'react'
import Layout from '@theme/Layout'
import { Hero } from '../components/Hero'
import useBaseUrl from '@docusaurus/useBaseUrl'
import styles from './index.module.css'
import Link from '@docusaurus/Link'
import { Arrow } from '../icons/icons'

const CARDS = [
  {
    color: 'blue',
    headline: 'How-to Guides',
    text:
      'These problem-oriented guides take the reader through steps required to solve a problem.',
    link: '/docs/how-to/overview',
  },
  {
    color: 'green',
    headline: 'Conceptual Guides',
    text:
      'These understanding-oriented guides clarify a particular topic by giving context and a wider view.',
    link: '/docs/conceptual/overview',
  },
  {
    color: 'purple',
    headline: 'Reference Docs',
    text:
      'These information-oriented docs provide technical descriptions of the code and how to operate it.',
    link: '/docs/reference/overview',
  },
  {
    color: 'orange',
    headline: 'Tutorials',
    text:
      'These learning-oriented lessons take the reader by the hand to complete a small project.',
    link: '/docs/tutorials/overview',
  },
]

const ESSENTIALS = [
  [
    {
      title: 'Install and start Actyx',
      link: '/docs/how-to/local-development/install-actyx',
    },
    {
      title: 'Set up a new project',
      link: '/docs/how-to/local-development/set-up-a-new-project',
    },
    {
      title: 'Package for mobile',
      link: '/docs/how-to/packaging/mobile-apps',
    },
    {
      title: 'Set up a swarm',
      link: '/docs/how-to/swarms/setup-swarm',
    },
  ],
  [
    {
      title: 'How Actyx works',
      link: '/docs/conceptual/how-actyx-works',
    },
    {
      title: 'Event-based systems',
      link: '/docs/conceptual/event-sourcing',
    },
    {
      title: 'Local First Cooperation',
      link: '/docs/conceptual/local-first-cooperation',
    },
    {
      title: 'Performance and limits',
      link: '/docs/conceptual/performance-and-limits',
    },
  ],
  [
    {
      title: 'Event Service API',
      link: '/docs/reference/events-api',
    },
    {
      title: 'Actyx reference',
      link: '/docs/reference/actyx-reference',
    },
    {
      title: 'Actyx CLI reference',
      link: '/docs/reference/cli/cli-overview',
    },
    {
      title: 'Node Manager reference',
      link: '/docs/reference/node-manager',
    },
  ],
]

const Card = ({ headline, text, link, color }) => (
  <div className={'col col--3 ' + styles.card + ' ' + styles.shiftOnHover + ' invert ' + color}>
    <Link to={link}>
      <div>
        <h3 className={styles.headline}>{headline}</h3>
        <p className={styles.text}>{text}</p>
        <p className={styles.link}>
          Find out more
          <Arrow />
        </p>
      </div>
    </Link>
  </div>
)

const EssentialLink = ({ title, link }) => (
  <Link className={styles.essentialLink} to={link}>
    <p className={styles.link}>
      {title}
      <Arrow />
    </p>
  </Link>
)

const Text = ({ children }) => <div className={styles.text}>{children}</div>
const Heading = ({ children }) => <h2 className={styles.headline}>{children}</h2>

const Row = ({ children, className = '' }) => <div className={`row ${className}`}>{children}</div>
const Col = ({ width, children, className = '' }) => (
  <div className={`col col--${width} ${className}`}>{children}</div>
)

const Home = () => (
  <div className={'container ' + styles.root}>
    <Hero img={useBaseUrl('/images/homepage/hero-background.svg')} />
    <Row>
      <Col width="6">
        <Heading>What does Actyx do?</Heading>
        <Text>
          <p>
            Actyx enables a new way of building software that runs across multiple networked
            devices. It uses edge devices' local capabilities and doesn't depend on any central
            components (servers, databases, or cloud).
          </p>
          <p>
            Small pieces of code — they are called <em>local-twins</em> — contain the logic. Any app
            on any device can summon any twin at any time. Actyx automatically synchronizes the
            twins within the local network.
          </p>
        </Text>
      </Col>
      <Col width="6">
        <Heading>Why should I use Actyx?</Heading>
        <Text>
          <p>
            Actyx lets you easily build resilient real-time applications that run across a network
            of devices; use if you:
          </p>
          <ul>
            <li>want to easily implement distributed processes</li>
            <li>dislike depending on central servers or the cloud</li>
            <li>strive to offer users 100% application uptime</li>
            <li>like remaining in control of your data and compute</li>
          </ul>
        </Text>
      </Col>
    </Row>
    <Row>
      <Col width="12" className={styles.narrowSpacing}>
        <Heading>Documentation</Heading>
      </Col>
    </Row>
    <Row className={styles.cards}>
      {CARDS.map((c) => (
        <Card key={c.link} {...c} />
      ))}
    </Row>
    <Row>
      <Col width="6" className={styles.essentialsHeader}>
        <Heading>Essentials</Heading>
        <Text>
          <p>
            Check out the following topics to learn the essentials of building, running, and
            deploying systems using Actyx.
          </p>
        </Text>
      </Col>
    </Row>
    <Row className={styles.essentials}>
      <Col width="3" className="blue">
        {ESSENTIALS[0].map((e) => (
          <EssentialLink key={e.link} {...e} />
        ))}
      </Col>
      <Col width="3" className="green">
        {ESSENTIALS[1].map((e) => (
          <EssentialLink key={e.link} {...e} />
        ))}
      </Col>
      <Col width="3" className="purple">
        {ESSENTIALS[2].map((e) => (
          <EssentialLink key={e.link} {...e} />
        ))}
      </Col>
      <Col
        width="3"
        className={
          styles.card + ' ' + styles.shiftOnHover + ' ' + styles.essentialCommunity + ' dark'
        }
      >
        <a href="https://community.actyx.com">
          <div>
            <h3 className={styles.headline}>Actyx Community</h3>
            <p className={styles.text}>Join our developer community and discover our forum.</p>
            <p className={styles.link}>
              Visit forum <Arrow />
            </p>
          </div>
        </a>
      </Col>
    </Row>
  </div>
)

function HomePage() {
  return (
    <Layout title="Actyx Developer Documentation" description="">
      <Home />
    </Layout>
  )
}

export default HomePage
