/* 
This file contains all categories and their contents of how-to guides.
*/

const createConceptuals = () => {
  const conceptualGuides = [
    {
      category: 'Building Event-based Systems',
      description:
        'Fundamental principles that apply when working with event-based systems, such as event-sourcing and event ordering using Lamport clocks.',
      link: '',
    },
    {
      category: 'Distributed systems architectures and caveats',
      description:
        'Fundamental principles that apply when working in distributed systems, such as eventual consistency and the CAP theorem.',
      link: '',
    },
    {
      category: 'Local-First Computing Paradigm',
      description: 'How the LFC paradigm works and how its principles apply to factory automation.',
      link: '',
    },
    {
      category: 'Thinking in Actyx',
      description:
        'How local twins are used conceptually to build resilient factory solutions and how they can be transferred to code.',
      link: '',
    },
    {
      category: 'Actyx Jargon',
      description:
        'Actyx has lots of amazing features that make factory automation easier. But, like any technology, that means there is some jargon that can be confusing to newcomers.',
      link: '',
    },
    {
      category: 'Actyx vs. the Cloud',
      description:
        'Key differences of edge and cloud computing and points where both technologies can effectively complement each other.',
      link: '',
    },
    {
      category: 'Peer Discovery',
      description:
        'In distributed systems, many nodes need to collaborate closely. How do they find and talk to each other?',
      link: '',
    },
    {
      category: 'Performance and limits of Actyx',
      description:
        'Supported hardware, architectures and operating systems. Infrastructure limitations in real-world factory solutions.',
      link: '',
    },
    {
      category: 'Security in Actyx',
      description: 'Security provided by Actyx for developers and IT admins.',
      link: '',
    },
    {
      category: 'The Actyx node',
      description: 'Architecture and key components of the Actyx node.',
      link: '',
    },
    {
      category: 'Actyx Lifecycle',
      description:
        'States, transitions and triggering events that can change the lifecycle of an Actyx node.',
      link: '',
    },
    {
      category: 'Apps in the factory context',
      description:
        'Key capabilities, use-cases, and differences of headless and front-end applications in the factory setting',
      link: '',
    },
  ]
  return conceptualGuides
}

const conceptualGuides = createConceptuals()
export default conceptualGuides
