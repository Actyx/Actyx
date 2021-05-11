/* 
This file contains all categories and their contents of how-to guides.
*/

const createReferences = () => {
  const references = [
    {
      title: 'ActyxOS Reference',
      description: 'Supported operating systems, archiectures, and the node settings schema.',
      link: 'actyx-reference',
    },
    {
      title: 'Event Service API',
      description:
        'The underlying HTTP API responsible for publishing, querying and subscribing to events.',
      link: 'events-api',
    },
    {
      title: 'Actyx Pond',
      description: 'A programming framework for writing distributed applications.',
      link: 'pond-api-reference',
    },
    {
      title: 'Javascript/Typescript SDK',
      description: 'The Javascript/Typescript SDK for the Event Service HTTP API.',
      link: 'js-ts-sdk',
    },
    {
      title: 'Actyx Node Manager',
      description: 'An easy-to-use GUI application for managing ActyxOS nodes.',
      link: 'node-manager',
    },
    {
      title: 'Actyx CLI',
      description: 'The command line interface for managing ActyxOS nodes.',
      link: 'cli/cli-overview',
    },
  ]
  return references
}

const referenceDocs = createReferences()
export default referenceDocs
