/* 
This file contains all categories and their contents of how-to guides.
*/

const createConceptuals = () => {
  const conceptualGuides = [
    {
      title: 'How Actyx works',
      description: 'A short introduction into how actyx works on a conceptual level.',
      link: 'how-actyx-works',
    },
    {
      title: 'Building Event-based Systems',
      description: 'Fundamental principles that apply when working with event-based systems.',
      link: 'event-sourcing',
    },
    {
      title: 'Distributed systems architectures and caveats',
      description:
        'Fundamental principles that apply when working in distributed systems, such as eventual consistency and the CAP theorem.',
      link: 'distributed-system-architectures',
    },
    {
      title: 'Local-First Cooperation Paradigm',
      description: 'How the LFC paradigm works and how its principles apply to factory automation.',
      link: 'local-first-cooperation',
    },
    /*     {
      title: 'Thinking in Actyx',
      description:
        'How local twins are used conceptually to build resilient factory solutions and how they can be transferred to code.',
      link: '',
    }, */
    {
      title: 'Actyx Jargon',
      description:
        'Actyx has lots of amazing features that make factory automation easier. But, like any technology, that means there is some jargon that can be confusing to newcomers.',
      link: 'actyx-jargon',
    },
    /*     {
      title: 'Actyx vs. the Cloud',
      description:
        'Key differences of edge and cloud computing and points where both technologies can effectively complement each other.',
      link: '',
    }, */
    {
      title: 'Peer Discovery',
      description:
        'In distributed systems, many nodes need to collaborate closely. How do they find and talk to each other?',
      link: 'peer-discovery',
    },
    {
      title: 'Performance and limits of Actyx',
      description: 'Performance limitations in real-world factory solutions.',
      link: 'performance-and-limits',
    },
    {
      title: 'Security in Actyx',
      description: 'Basic netowrk security guarantees provided by ActyxOS.',
      link: 'security-in-actyx',
    },
    {
      title: 'ActyxOS components',
      description: 'Architecture and components of ActyxOS.',
      link: 'the-actyx-node',
    },
    {
      title: 'ActyxOS node and app lifecycle',
      description: 'Lifecycle of ActyxOS and apps deployed to ActyxOS runtimes.',
      link: 'actyx-node-lifecycle',
    },
    /*     {
      title: 'Apps in the factory context',
      description:
        'Key capabilities, use-cases, and differences of headless and front-end applications in the factory setting',
      link: '',
    }, */
  ]
  return conceptualGuides
}

const conceptualGuides = createConceptuals()
export default conceptualGuides
