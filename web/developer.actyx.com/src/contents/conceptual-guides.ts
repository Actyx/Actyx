/* 
This file contains all categories and their contents of how-to guides.
*/

const createConceptuals = () => {
  const conceptualGuides = [
    {
      title: 'Building Event-based Systems',
      description: 'Fundamental principles that apply when working with event-based systems.',
      link: '',
    },
    {
      title: 'Distributed systems architectures and caveats',
      description:
        'Fundamental principles that apply when working in distributed systems, such as eventual consistency and the CAP theorem.',
      link: '',
    },
    {
      title: 'Local-First Cooperation Paradigm',
      description: 'How the LFC paradigm works and how its principles apply to factory automation.',
      link: '',
    },
    {
      title: 'Actyx Jargon',
      description:
        'Actyx has lots of amazing features that make factory automation easier. But, like any technology, that means there is some jargon that can be confusing to newcomers.',
      link: '',
    },
    {
      title: 'Peer Discovery',
      description:
        'In distributed systems, many nodes need to collaborate closely. How do they find and talk to each other?',
      link: '',
    },
    {
      title: 'Performance and limits of Actyx',
      description: 'Performance limitations in real-world factory solutions.',
      link: '',
    },
    {
      title: 'Security in Actyx',
      description: 'Basic netowrk security guarantees provided by ActyxOS.',
      link: '',
    },
    {
      title: 'ActyxOS components',
      description: 'Architecture and components of ActyxOS.',
      link: '',
    },
    {
      title: 'ActyxOS node and app lifecycle',
      description: 'Lifecycle of ActyxOS and apps deployed to ActyxOS runtimes.',
      link: '',
    },
  ]
  return conceptualGuides
}

const conceptualGuides = createConceptuals()
export default conceptualGuides
